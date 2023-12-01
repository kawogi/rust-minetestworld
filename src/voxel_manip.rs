//! Contains a type to more high-level world reading and writing

use std::collections::HashMap;
use std::{collections::hash_map::Entry, sync::Arc};

use async_std::sync::Mutex;
use glam::I16Vec3;

use crate::positions::NodePos;
use crate::{
    positions::{BlockPos, SplitPos},
    MapBlock, MapData, MapDataError, Node,
};
type Result<T> = std::result::Result<T, MapDataError>;

struct BlockEdit {
    mapblock: MapBlock,
    tainted: bool,
}

impl BlockEdit {
    /// Get the node at the given world position
    pub fn get_node(&self, node_pos: NodePos) -> Node {
        self.mapblock.get_node_at(node_pos)
    }

    /// Set a voxel in VoxelManip's cache
    ///
    /// ⚠️ The change will be present locally only. To modify the map,
    /// the change has to be written back via [`VoxelManip::commit`].
    pub fn set_node(&mut self, node_pos: NodePos, node: Node) {
        let content_id = self.mapblock.get_or_create_content_id(&node.param0);
        self.mapblock.set_content(node_pos, content_id);
        self.mapblock.set_param1(node_pos, node.param1);
        self.mapblock.set_param2(node_pos, node.param2);
        self.tainted = true;
    }

    /// Sets the content string at this world position
    ///
    /// `content` has to be the unique [itemstring](https://wiki.minetest.net/Itemstrings).
    /// The use of aliases is not possible, because it would require a Lua runtime
    /// loading all mods.
    ///
    /// ```ignore
    /// vm.set_content(Position::new(8,9,10), b"default:stone").await?;
    /// ```
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub fn set_content(&mut self, node_pos: NodePos, content: &[u8]) {
        let content_id = self.mapblock.get_or_create_content_id(content);
        self.mapblock.set_content(node_pos, content_id);
        self.tainted = true;
    }

    /// Sets the lighting parameter at this world position
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub fn set_param1(&mut self, node_pos: NodePos, param1: u8) {
        self.mapblock.set_param1(node_pos, param1);
        self.tainted = true;
    }

    /// Sets the param2 of the node at this world position
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub fn set_param2(&mut self, node_pos: NodePos, param2: u8) {
        self.mapblock.set_param2(node_pos, param2);
        self.tainted = true;
    }
}

/// In-memory world data cache that allows easy handling of single nodes.
///
/// It is an abstraction on top of the MapBlocks the world data consists of.
/// It allows fast reading from and writing to the world.
///
/// All changes to the world have to be committed via [`VoxelManip::commit`].
/// Before this, they are only present in VoxelManip's local cache and lost after drop.
///
/// ⚠️ You want to do a world backup before modifying the map data.
pub struct MapEdit {
    map: MapData,
    mapblock_cache: HashMap<BlockPos, Arc<async_std::sync::Mutex<BlockEdit>>>,
}

impl MapEdit {
    /// Create a new VoxelManip from a handle to a map data backend
    pub fn new(map: MapData) -> Self {
        MapEdit {
            map,
            mapblock_cache: HashMap::new(),
        }
    }

    /// Return a cache entry containing the given mapblock
    async fn get_mapblock(&mut self, mapblock_pos: BlockPos) -> Result<Arc<Mutex<BlockEdit>>> {
        // if let Some(occupied) = self.mapblock_cache.get(&mapblock_pos) {
        //     return Ok(occupied.lock());
        // }
        // {
        //     let mapblock = match self.map.get_mapblock(mapblock_pos).await {
        //         Ok(mapblock) => Ok(mapblock),
        //         Err(MapDataError::MapBlockNonexistent(_)) => Ok(MapBlock::unloaded()),
        //         Err(e) => Err(e),
        //     }?;

        //     let v = Arc::new(Mutex::new(BlockEdit {
        //         mapblock,
        //         tainted: false,
        //     }));

        //     self.mapblock_cache.insert(mapblock_pos, v);

        //     todo!()
        // }
        //  Ok(self.mapblock_cache.get(&mapblock_pos).unwrap().lock())
        let c = match self.mapblock_cache.entry(mapblock_pos) {
            Entry::Occupied(e) => {
                //
                let block = e.get();
                block.clone()
            }
            Entry::Vacant(e) => {
                // If not in the database, create unloaded mapblock
                let mapblock = match self.map.get_mapblock(mapblock_pos).await {
                    Ok(mapblock) => Ok(mapblock),
                    Err(MapDataError::MapBlockNonexistent(_)) => Ok(MapBlock::unloaded()),
                    Err(e) => Err(e),
                }?;
                let block = e.insert(Arc::new(Mutex::new(BlockEdit {
                    mapblock,
                    tainted: false,
                })));

                block.clone()
            }
        };

        Ok(c)
    }

    // /// Get a reference to the mapblock at the given block position
    // ///
    // /// If there is no mapblock at this world position,
    // /// a new [unloaded](`MapBlock::unloaded`) mapblock is returned.
    // pub async fn get_mapblock(&mut self, mapblock_pos: BlockPos) -> Result<MutexGuard<BlockEdit>> {
    //     let get_entry = self.get_entry(mapblock_pos).await?;
    //     Ok(get_entry.await?.lock().await)
    // }

    /// Get the node at the given world position
    pub async fn get_node(&mut self, node_pos: I16Vec3) -> Result<Node> {
        let (blockpos, nodepos) = node_pos.split();
        Ok(self
            .get_mapblock(blockpos)
            .await?
            .lock()
            .await
            .get_node(nodepos))
    }

    /// Do something with the mapblock at `blockpos` and mark it as modified
    async fn modify_mapblock(
        &mut self,
        blockpos: BlockPos,
        op: impl FnOnce(&mut BlockEdit),
    ) -> Result<()> {
        let entry = &mut self.get_mapblock(blockpos).await?;
        let mut block_edit = entry.lock().await;
        op(&mut block_edit);
        block_edit.tainted = true;
        Ok(())
    }

    /// Set a voxel in VoxelManip's cache
    ///
    /// ⚠️ The change will be present locally only. To modify the map,
    /// the change has to be written back via [`VoxelManip::commit`].
    pub async fn set_node(&mut self, node_pos: I16Vec3, node: Node) -> Result<()> {
        let (blockpos, nodepos) = node_pos.split();
        let mutex = &self.get_mapblock(blockpos).await?;
        let mut block_edit = mutex.lock().await;
        block_edit.set_node(nodepos, node);
        Ok(())
    }

    /// Sets the content string at this world position
    ///
    /// `content` has to be the unique [itemstring](https://wiki.minetest.net/Itemstrings).
    /// The use of aliases is not possible, because it would require a Lua runtime
    /// loading all mods.
    ///
    /// ```ignore
    /// vm.set_content(Position::new(8,9,10), b"default:stone").await?;
    /// ```
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub async fn set_content(&mut self, node_pos: I16Vec3, content: &[u8]) -> Result<()> {
        let (blockpos, nodepos) = node_pos.split();
        let mutex = &self.get_mapblock(blockpos).await?;
        let mut block_edit = mutex.lock().await;
        block_edit.set_content(nodepos, content);
        Ok(())
    }

    /// Sets the lighting parameter at this world position
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub async fn set_param1(&mut self, node_pos: I16Vec3, param1: u8) -> Result<()> {
        let (blockpos, nodepos) = node_pos.split();
        let mutex = &self.get_mapblock(blockpos).await?;
        let mut block_edit = mutex.lock().await;
        block_edit.set_param1(nodepos, param1);
        Ok(())
    }

    /// Sets the param2 of the node at this world position
    ///
    /// ⚠️ Until the change is [commited](`VoxelManip::commit`),
    /// the node will only be changed in the cache.
    pub async fn set_param2(&mut self, node_pos: I16Vec3, param2: u8) -> Result<()> {
        let (blockpos, nodepos) = node_pos.split();
        let mutex = &self.get_mapblock(blockpos).await?;
        let mut block_edit = mutex.lock().await;
        block_edit.set_param2(nodepos, param2);
        Ok(())
    }

    /// Returns true if this world position is cached
    pub fn is_in_cache(&self, node_pos: I16Vec3) -> bool {
        let (blockpos, _) = node_pos.split();
        self.mapblock_cache.contains_key(&blockpos)
    }

    /// Ensures that this world position is in the cache
    pub async fn visit(&mut self, node_pos: I16Vec3) -> Result<()> {
        let (blockpos, _) = node_pos.split();
        self.get_mapblock(blockpos).await?;
        Ok(())
    }

    /// Apply all changes made to the map
    ///
    /// Without this, all changes made with [`VoxelManip::set_node`], [`VoxelManip::set_content`],
    /// [`VoxelManip::set_param1`], and [`VoxelManip::set_param2`] are lost when this
    /// instance is dropped.
    pub async fn commit(&mut self) -> Result<()> {
        // Write modified mapblocks back into the map data
        for (&pos, cache_entry) in self.mapblock_cache.iter_mut() {
            let mut cache_entry = cache_entry.lock().await;
            if cache_entry.tainted {
                self.map.set_mapblock(pos, &cache_entry.mapblock).await?;
                cache_entry.tainted = false;
            }
        }

        Ok(())
    }
}
