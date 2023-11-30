use crate::positions::BlockKey;
use crate::positions::BlockPos;
use crate::positions::NodeIndex;
use crate::positions::NodePos;
use crate::positions::SplitPos;
use crate::world::keyvalue_to_uri_connectionstr;
use crate::MapBlock;
use crate::MapData;
use crate::MapDataError;
use crate::World;
use crate::NODE_BITS_1D;
use futures::prelude::*;
use glam::I16Vec3;
use glam::U16Vec3;

#[test]
fn simple_math() {
    assert_eq!(
        BlockPos::from(BlockKey::try_from(134270984).unwrap()),
        BlockPos::from_index_vec(I16Vec3::new(8, 13, 8)),
    );
    assert_eq!(
        BlockPos::from(BlockKey::try_from(-184549374).unwrap()),
        BlockPos::from_index_vec(I16Vec3::new(2, 0, -11)),
    );
}

#[async_std::test]
async fn db_exists() {
    MapData::from_sqlite_file("TestWorld/map.sqlite", true)
        .await
        .unwrap();
}

#[async_std::test]
async fn can_query() {
    let mapdata = MapData::from_sqlite_file("TestWorld/map.sqlite", true)
        .await
        .unwrap();
    assert_eq!(mapdata.all_mapblock_positions().await.count().await, 5923);
    let block = mapdata
        .get_block_data((I16Vec3::new(-13, -8, 2) << NODE_BITS_1D).split().0)
        .await
        .unwrap();
    assert_eq!(block.len(), 40);
}

#[async_std::test]
async fn mapblock_miss() {
    let position = I16Vec3::new(0, 0, 0).split().0;
    let mapdata = MapData::from_sqlite_file("TestWorld/map.sqlite", true)
        .await
        .unwrap();
    let result = mapdata.get_mapblock(position).await;
    if let Err(MapDataError::MapBlockNonexistent(pos)) = result {
        assert_eq!(pos, position);
    } else {
        panic!("A missing map block should result in MapDataError::MapBlockNonexistent")
    }
}

#[test]
fn can_parse_mapblock() {
    MapBlock::from_data(std::fs::File::open("TestWorld/testmapblock").unwrap()).unwrap();
}

#[async_std::test]
async fn can_parse_all_mapblocks() {
    let mapdata = MapData::from_sqlite_file("TestWorld/map.sqlite", true)
        .await
        .unwrap();
    let positions: Vec<_> = mapdata
        .all_mapblock_positions()
        .await
        .try_collect()
        .await
        .unwrap();
    let blocks: Vec<_> =
        futures::future::join_all(positions.iter().map(|pos| mapdata.get_mapblock(*pos))).await;
    let succeeded = blocks.iter().filter(|b| b.is_ok()).count();
    let failed = blocks.iter().filter(|b| b.is_err()).count();
    eprintln!("Succeeded parsed blocks: {succeeded}\nFailed blocks: {failed}");
    assert_eq!(failed, 0);
}

#[async_std::test]
async fn count_nodes() {
    let blockpos = BlockPos::from_index_vec(I16Vec3::new(-13, -8, 2));

    let mapdata = MapData::from_sqlite_file("TestWorld/map.sqlite", true)
        .await
        .unwrap();
    let count = mapdata.iter_mapblock_nodes(blockpos).await.unwrap().count();
    assert_eq!(count, 4096);
}

#[async_std::test]
async fn iter_node_positions() {
    let blockpos = BlockPos::from_index_vec(I16Vec3::new(-13, -8, 2));

    let world = World::open("TestWorld");
    let mapdata = world.get_map_data().await.unwrap();
    for (pos, node) in mapdata.iter_mapblock_nodes(blockpos).await.unwrap() {
        println!("{pos:?}, {node:?}");
    }
}

#[test]
fn node_index() {
    assert_eq!(
        NodePos::from(NodeIndex::try_from(0).unwrap()),
        NodePos::try_from(U16Vec3::new(0, 0, 0)).unwrap()
    );
    assert_eq!(
        NodePos::from(NodeIndex::try_from(4095).unwrap()),
        NodePos::try_from(U16Vec3::new(15, 15, 15)).unwrap()
    );
}

#[test]
fn url_default_host() {
    assert_eq!(
        keyvalue_to_uri_connectionstr(""),
        Ok("postgresql://localhost:5432".to_string())
    );
}

#[test]
fn url_malformed_port() {
    assert!(keyvalue_to_uri_connectionstr("port=ß").is_err());
}

#[test]
fn url_nondefault_values() {
    assert_eq!(
        keyvalue_to_uri_connectionstr("port=15432 host=localhorst dbname=mtdb user=u password=p"),
        Ok("postgresql://u:p@localhorst:15432/mtdb".to_string())
    );
}
