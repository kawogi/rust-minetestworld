//! Functions and datatypes to work with world coordinates

use glam::{I16Vec3, I64Vec3, IVec3, U16Vec3};
#[cfg(feature = "postgres")]
use sqlx::postgres::PgRow;
#[cfg(feature = "sqlite")]
use sqlx::sqlite::SqliteRow;
#[cfg(any(feature = "sqlite", feature = "postgres"))]
use sqlx::{FromRow, Row};
use std::{fmt::Display, io};

use crate::{
    BLOCK_BITS_1D, BLOCK_KEY_MIN, BLOCK_KEY_RANGE, BLOCK_MASK, BLOCK_NODES_1D, BLOCK_NODES_3D,
    NODE_BITS_1D, NODE_MASK, WORLD_BLOCKS_RANGE,
};

fn invalid_data_error<E>(error: E) -> sqlx::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    sqlx::Error::Io(io::Error::new(io::ErrorKind::InvalidData, error))
}

/// A point location within the world
///
/// This type is used for addressing one of the following:
/// * voxels ([nodes](`crate::Node`), node timers, metadata, ...).
/// * [MapBlocks](`crate::MapBlock`). In this case, all three dimensions are divided by the
/// MapBlock [side length](`crate::MAPBLOCK_LENGTH`).
///
/// A voxel position may either be absolute or relative to a mapblock root.
///
/// - `x`: "East direction". The direction in which the sun rises.
/// - `y`: "Up" direction
/// - `z`: "North" direction. 90° left from the direction the sun rises.
// #[repr(transparent)]
// #[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
// pub struct WorldPos(pub I16Vec3);

#[repr(transparent)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub struct BlockPos(I16Vec3);

#[repr(transparent)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct BlockKey(i64);

impl From<BlockKey> for i64 {
    fn from(value: BlockKey) -> Self {
        value.0
    }
}

impl TryFrom<i64> for BlockKey {
    type Error = ();

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        if BLOCK_KEY_RANGE.contains(&value) {
            Ok(Self(value))
        } else {
            Err(())
        }
    }
}

impl BlockPos {
    pub fn join(self, node_pos: NodePos) -> I16Vec3 {
        I16Vec3::join(self, node_pos)
    }

    #[must_use]
    pub fn into_index_vec(self) -> I16Vec3 {
        self.0 >> NODE_BITS_1D
    }

    #[must_use]
    pub fn from_index_vec(vec: I16Vec3) -> Self {
        Self(vec << NODE_BITS_1D)
    }
}

impl From<BlockKey> for BlockPos {
    fn from(value: BlockKey) -> Self {
        // move values into positive range so that we no longer have to deal with sign bit overlapping
        // i will be in the range 0..(4096 * 4096 * 4096)
        // wrapping will never occur with the current bit sizes but it's still cleaner
        let i = value.0.wrapping_sub(BLOCK_KEY_MIN);

        // right-align coordinate bits
        // (sign-exteded but irrelevant due to i being positive now);
        // fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210
        // key: 0000000000000000000000000000zzzzzzzzzzzzyyyyyyyyyyyyxxxxxxxxxxxx
        //   x: 0000000000000000000000000000zzzzzzzzzzzzyyyyyyyyyyyyxxxxxxxxxxxx
        //   y: 0000000000000000000000000000000000000000zzzzzzzzzzzzyyyyyyyyyyyy
        //   x: 0000000000000000000000000000000000000000000000000000zzzzzzzzzzzz
        let lsb_aligned = I64Vec3::new(i, i >> BLOCK_BITS_1D, i >> (BLOCK_BITS_1D * 2));

        // truncate to 16 bits
        //   x: yyyyxxxxxxxxxxxx
        //   y: zzzzyyyyyyyyyyyy
        //   x: 0000zzzzzzzzzzzz
        let truncated = lsb_aligned.as_i16vec3();

        // left-align to remove MSB garbage and make room for the node position within this block
        //   x: xxxxxxxxxxxx0000
        //   y: yyyyyyyyyyyy0000
        //   x: zzzzzzzzzzzz0000
        let msb_aligned = truncated << NODE_BITS_1D;

        // re-introduce negative values by rotating tha value range back
        // (this undoes the initial transformation)
        Self(msb_aligned.wrapping_add(I16Vec3::splat(i16::MIN)))
    }
}

impl TryFrom<I16Vec3> for BlockPos {
    type Error = NodeIndexOutOfRange;

    fn try_from(value: I16Vec3) -> Result<Self, Self::Error> {
        if WORLD_BLOCKS_RANGE.contains(&value.x)
            && WORLD_BLOCKS_RANGE.contains(&value.y)
            && WORLD_BLOCKS_RANGE.contains(&value.z)
        {
            Ok(Self(value))
        } else {
            Err(NodeIndexOutOfRange)
        }
    }
}

impl From<BlockPos> for BlockKey {
    fn from(value: BlockPos) -> Self {
        let temp = (value.0 >> NODE_BITS_1D).as_i64vec3();
        Self(temp.x + (temp.y << BLOCK_BITS_1D) + (temp.z << (BLOCK_BITS_1D * 2)))
    }
}

#[cfg(feature = "sqlite")]
impl FromRow<'_, SqliteRow> for BlockPos {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(BlockKey(row.try_get::<i64, _>("pos")?).into())
    }
}

/// It is guaranteed that only the lowest `NODE_BITS_1D` bits are set
#[repr(transparent)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub struct NodePos(U16Vec3);

#[repr(transparent)]
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash, PartialOrd, Ord)]
pub struct NodeIndex(u16);

#[derive(Debug)]
pub struct NodeIndexOutOfRange;

impl TryFrom<u16> for NodeIndex {
    type Error = NodeIndexOutOfRange;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value < BLOCK_NODES_3D {
            Ok(Self(value))
        } else {
            Err(NodeIndexOutOfRange)
        }
    }
}

impl TryFrom<U16Vec3> for NodePos {
    type Error = NodeIndexOutOfRange;

    fn try_from(value: U16Vec3) -> Result<Self, Self::Error> {
        if value.x < BLOCK_NODES_1D && value.y < BLOCK_NODES_1D && value.z < BLOCK_NODES_1D {
            Ok(Self(value))
        } else {
            Err(NodeIndexOutOfRange)
        }
    }
}

impl From<NodeIndex> for u16 {
    fn from(value: NodeIndex) -> Self {
        value.0
    }
}

impl From<NodePos> for U16Vec3 {
    fn from(value: NodePos) -> Self {
        value.0
    }
}

impl Display for NodeIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Display for BlockKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Convert a nodex index (used in flat 16·16·16 arrays) into a node position
///
/// The node position will be relative to the map block.
impl From<NodeIndex> for NodePos {
    fn from(node_index: NodeIndex) -> Self {
        // ....zzzzyyyyxxxx
        Self(U16Vec3::new(
            node_index.0 & NODE_MASK,
            (node_index.0 >> NODE_BITS_1D) & NODE_MASK,
            (node_index.0 >> (NODE_BITS_1D * 2)) & NODE_MASK,
        ))
    }
}

/// Convert a MapBlock-relative node position into a flat array index
impl From<NodePos> for NodeIndex {
    fn from(value: NodePos) -> NodeIndex {
        Self(value.0.x + (value.0.y << 4) + (value.0.z << 8))
    }
}

impl From<NodeIndex> for usize {
    fn from(value: NodeIndex) -> usize {
        usize::from(value.0)
    }
}

impl From<NodePos> for usize {
    fn from(value: NodePos) -> usize {
        NodeIndex::from(value).into()
    }
}

// impl std::ops::Add for WorldPos {
//     type Output = Self;

//     fn add(self, rhs: Self) -> Self {
//         Self(self.0 + rhs.0)
//     }
// }

// impl std::ops::Sub for WorldPos {
//     type Output = Self;

//     fn sub(self, rhs: Self) -> Self::Output {
//         Self(self.0 - rhs.0)
//     }
// }

// impl std::ops::Mul<i16> for WorldPos {
//     type Output = Self;

//     fn mul(self, rhs: i16) -> Self {
//         Self(self.0 * rhs)
//     }
// }

// impl From<I16Vec3> for WorldPos {
//     fn from(value: I16Vec3) -> Self {
//         WorldPos(value)
//     }
// }

// impl From<WorldPos> for I16Vec3 {
//     fn from(value: WorldPos) -> Self {
//         value.0
//     }
// }

#[cfg(feature = "postgres")]
impl FromRow<'_, PgRow> for BlockPos {
    /// Will fail if one of the pos components do not fit in an i16
    fn from_row(row: &PgRow) -> sqlx::Result<Self> {
        IVec3::new(
            row.try_get("posx")?,
            row.try_get("posy")?,
            row.try_get("posz")?,
        )
        .try_into()
        .map(Self)
        .map_err(invalid_data_error)
    }
}

// /// While there is no modulo operator in rust, we'll use the remainder operator (%) to build one.
// pub fn modulo<I>(a: I, b: I) -> I
// where
//     I: Copy + Add<Output = I> + Rem<Output = I>,
// {
//     (a % b + b) % b
// }

// impl WorldPos {
//     /// Create a new position value from its components
//     pub fn new<I: Into<i16>>(x: I, y: I, z: I) -> Self {
//         I16Vec3::new(x.into(), y.into(), z.into()).into()
//     }

//     /// Return the mapblock position corresponding to this node position
//     pub fn mapblock_at(&self) -> WorldPos {
//         WorldPos::new(
//             div_floor(self.0.x, MAPBLOCK_LENGTH.into()),
//             div_floor(self.0.y, MAPBLOCK_LENGTH.into()),
//             div_floor(self.0.z, MAPBLOCK_LENGTH.into()),
//         )
//     }

//     /// Split this node position into a mapblock position and a relative node position
//     pub fn split_at_block(&self) -> (WorldPos, WorldPos) {
//         let blockpos = self.mapblock_at();
//         let relative_pos = *self - blockpos * MAPBLOCK_LENGTH as i16;
//         (blockpos, relative_pos)
//     }
// }

pub trait SplitPos {
    fn split(self) -> (BlockPos, NodePos);
    fn join(block_pos: BlockPos, node_pos: NodePos) -> Self;
}

impl SplitPos for I16Vec3 {
    fn split(self) -> (BlockPos, NodePos) {
        (
            BlockPos(self & I16Vec3::splat(BLOCK_MASK)),
            NodePos(self.as_u16vec3() & U16Vec3::splat(NODE_MASK)),
        )
    }

    fn join(block_pos: BlockPos, node_pos: NodePos) -> Self {
        block_pos.0 + node_pos.0.as_i16vec3()
    }
}
