use glam::{I16Vec3, U16Vec3};
use minetestworld::{
    positions::{BlockKey, BlockPos, NodeIndex, SplitPos},
    BLOCK_BITS_1D, BLOCK_KEY_MAX, BLOCK_KEY_MIN, BLOCK_KEY_RANGE,
};
use rand::Rng;

fn test_vec(world_pos: I16Vec3) {
    let (block_pos, node_pos) = world_pos.split();
    let block_index = BlockKey::from(block_pos);
    let node_index = NodeIndex::from(node_pos);
    println!(
        "{world_pos} = {block_pos} [{block_index}] + {node_pos} [{node_index}]",
        block_pos = block_pos.into_index_vec(),
        node_pos = U16Vec3::from(node_pos)
    );
}

#[async_std::main]
async fn main() {
    let mut rng = rand::thread_rng();
    for _ in 0..1_000_000 {
        let i = rng.gen_range(BLOCK_KEY_RANGE);
        let pos = BlockPos::from(BlockKey::try_from(i).unwrap());
        let index = i64::from(BlockKey::from(pos));
        assert_eq!(i, index);

        let x = rng.gen();
        let y = rng.gen();
        let z = rng.gen();
        let v = I16Vec3::new(x, y, z);
        let (block, node) = v.split();
        let index = i64::from(BlockKey::from(block));
        let block2 = BlockPos::from(BlockKey::try_from(index).unwrap());
        let v2 = I16Vec3::join(block2, node);
        assert_eq!(v, v2);
    }

    println!(
        "{}",
        BlockPos::from(BlockKey::try_from(BLOCK_KEY_MIN).unwrap()).into_index_vec()
    );

    println!(
        "{}",
        BlockPos::from(BlockKey::try_from(BLOCK_KEY_MAX).unwrap()).into_index_vec()
    );

    assert!(BlockKey::try_from(BLOCK_KEY_MIN - 1).is_err());
    assert!(BlockKey::try_from(BLOCK_KEY_MAX + 1).is_err());

    test_vec(I16Vec3::new(8, 13, 8));
    test_vec(I16Vec3::new(2, 0, -11));
    test_vec(I16Vec3::new(8, 13, 8) << BLOCK_BITS_1D);
    test_vec(I16Vec3::new(2, 0, -11) << BLOCK_BITS_1D);

    println!();
    let key = BlockKey::try_from(134270984).unwrap();
    let block_pos = BlockPos::from(key);
    let key2 = BlockKey::from(block_pos);
    println!(
        "{key} → {block_pos:?} → {key2}",
        block_pos = block_pos.into_index_vec()
    );

    println!();
    let key = BlockKey::try_from(-184549374).unwrap();
    let block_pos = BlockPos::from(key);
    let key2 = BlockKey::from(block_pos);
    println!(
        "{key} → {block_pos:?} → {key2}",
        block_pos = block_pos.into_index_vec()
    );

    // assert_eq!(
    //     BlockPos::from(BlockKey::try_from(134270984).unwrap()),
    //     BlockPos::try_from(I16Vec3::new(8, 13, 8)).unwrap(),
    // );
    // assert_eq!(
    //     BlockPos::from(BlockKey::try_from(-184549374).unwrap()),
    //     BlockPos::try_from(I16Vec3::new(2, 0, -11)).unwrap(),
    // );

    for z in -2..2 {
        for y in -2..2 {
            for x in -2..2 {
                test_vec(I16Vec3::new(x, y, z));
            }
        }
    }
}
