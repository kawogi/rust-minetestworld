//! This crate lets you read the world data of a minetest world.
//!
//! Only map format version 29 is supported. LevelDB backend is not supported.
//!
//! ## Terminology
//! ### Node
//! [Nodes](`Node`) are the single voxels that the world data consist of. It has three properties:
//! 1. A content type string (like `air` or `default:dirt`)
//! 2. Flags to determine lighting rendering
//! 3. Additional data that can be interpreted based on the content type (e.g. flow information for liquids)
//!
//! This term might originate in the Irrlicht engine.
//!
//! ### MapBlock
//! When saved in a backend, the world data is divided into chunks that are called
//! [map blocks](`MapBlock`). A map block contains 16·16·16 nodes as well as objects and metadata.
//!
//! A mapblock is addressed by a [`Position`] where every dimension
//! is divided by [`MAPBLOCK_LENGTH`].
//!
//! ## Example usage
//!
//! An example that reads all nodes of a specific map block:
//! ```
//! use minetestworld::{World, positions::BlockPos};
//! use glam::I16Vec3;
//! use async_std::task;
//!
//! let blockpos = BlockPos::from_index_vec(I16Vec3::new(-13, -8, 2));
//!
//! task::block_on(async {
//!     let world = World::open("TestWorld");
//!     let mapdata = world.get_map_data().await.unwrap();
//!     for (pos, node) in mapdata.iter_mapblock_nodes(blockpos).await.unwrap() {
//!         println!("{pos:?}, {node:?}");
//!     }
//! });
//! ```
//!
//! [Another notable example](https://docs.rs/crate/minetestworld/latest/source/examples/modify_map.rs)
//! uses a [`VoxelManip`] to modify the world.
#![warn(missing_docs)]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

extern crate async_std;
#[cfg(feature = "smartstring")]
extern crate smartstring;

pub mod map_block;
pub mod map_data;
pub mod positions;
pub mod voxel_manip;
pub mod world;

use std::ops::Range;

pub use map_block::MapBlock;
pub use map_block::Node;
pub use map_data::MapData;
pub use map_data::MapDataError;
pub use voxel_manip::VoxelManip;
pub use world::World;
pub use world::WorldError as Error;

// /// Side length of map blocks.
// ///
// /// The map data is divided into chunks of nodes (=voxels).
// /// Currently, a chunk consists of 16·16·16 nodes.
// ///
// /// The number of nodes a mapblock contains is [`MAPBLOCK_SIZE`].
// ///
// /// ```
// /// use minetestworld::MAPBLOCK_LENGTH;
// ///
// /// assert_eq!(MAPBLOCK_LENGTH, 16);
// /// ```
// pub const MAPBLOCK_LENGTH: u8 = 16;

// /// How many nodes are contained in a map block.
// ///
// /// This is [`MAPBLOCK_LENGTH`]³.
// ///
// /// ```
// /// use minetestworld::MAPBLOCK_SIZE;
// ///
// /// assert_eq!(MAPBLOCK_SIZE, 4096);
// /// ```
// pub const MAPBLOCK_SIZE: usize =
//     MAPBLOCK_LENGTH as usize * MAPBLOCK_LENGTH as usize * MAPBLOCK_LENGTH as usize;

/// Number of bits needed to address all nodes within a world in each dimension
pub const WORLD_BITS_1D: u32 = i16::BITS;

/// Number of bits needed to address nodes within a block in each dimension
pub const NODE_BITS_1D: u32 = 4;

/// The bits needed to address nodes within a block
pub const NODE_MASK: u16 = (1 << NODE_BITS_1D) - 1;

/// Number of bits needed to address blocks within a world in each dimension
pub const BLOCK_BITS_1D: u32 = WORLD_BITS_1D - NODE_BITS_1D;

/// The bits needed to address blocks within a world
pub const BLOCK_MASK: i16 = -1 << NODE_BITS_1D;

/// Number of bits needed to address all nodes within a world
pub const WORLD_BITS_3D: u32 = WORLD_BITS_1D * 3;

/// Number of bits needed to address nodes within a block
pub const NODE_BITS_3D: u32 = NODE_BITS_1D * 3;

/// Number of bits needed to address blocks within a world
pub const BLOCK_BITS_3D: u32 = BLOCK_BITS_1D * 3;

/// Number of nodes in an entire world in each dimension
pub const WORLD_NODES_1D: u32 = 1 << WORLD_BITS_1D;

/// Number of nodes per block in each dimension
pub const BLOCK_NODES_1D: u16 = 1 << NODE_BITS_1D;

/// Number of nodes per block in each dimension
pub const WORLD_BLOCKS_1D: u16 = 1 << BLOCK_BITS_1D;

/// Minimum block index for all dimensions
pub const WORLD_BLOCKS_MIN: i16 = -1 << (BLOCK_BITS_1D - 1);

/// Maximum block index for all dimensions
pub const WORLD_BLOCKS_MAX: i16 = (1 << (BLOCK_BITS_1D - 1)) - 1;

/// Valid block index range for all dimensions
pub const WORLD_BLOCKS_RANGE: Range<i16> = WORLD_BLOCKS_MIN..(1 << (BLOCK_BITS_1D - 1));

const DIAGONAL_KEY_STRIDE: i64 =
    1 + WORLD_BLOCKS_1D as i64 + WORLD_BLOCKS_1D as i64 * WORLD_BLOCKS_1D as i64;

pub const BLOCK_KEY_MIN: i64 = WORLD_BLOCKS_MIN as i64 * DIAGONAL_KEY_STRIDE;
pub const BLOCK_KEY_MAX: i64 = WORLD_BLOCKS_MAX as i64 * DIAGONAL_KEY_STRIDE;
pub const BLOCK_KEY_RANGE: Range<i64> = BLOCK_KEY_MIN..(BLOCK_KEY_MAX + 1);

/// Number of nodes in an entire world
pub const WORLD_NODES_3D: u64 = 1 << WORLD_BITS_3D;

/// Number of nodes in an entire block
pub const BLOCK_NODES_3D: u16 = 1 << NODE_BITS_3D;

/// Number of nodes in an entire block as usize for convenience
pub const BLOCK_NODES_3D_U: usize = BLOCK_NODES_3D as usize;

/// Number of blocks in an entire world
pub const WORLD_BLOCKS_3D: u64 = 1 << BLOCK_BITS_3D;

/// Number of blocks in an entire world as usize for convenience
pub const WORLD_BLOCKS_3D_U: usize = WORLD_BLOCKS_3D as usize;

#[cfg(test)]
mod tests;
