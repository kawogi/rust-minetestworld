use std::error::Error;
mod common;
use glam::I16Vec3;
use minetestworld::World;

async fn change_voxel() -> Result<(), minetestworld::world::WorldError> {
    let world = World::open("TestWorld copy");
    let pos = I16Vec3::new(0, 0, 0);
    
    let mut vm = world.get_voxel_manip(true).await?;
    vm.set_content(pos, b"default:diamond").await?;
    let node = vm.get_node(pos).await?;
    assert_eq!(node.param0, b"default:diamond");

    vm.commit().await?;
    std::mem::drop(vm);

    let mut vm = world.get_voxel_manip(true).await?;
    let node = vm.get_node(pos).await?;
    assert_eq!(node.param0, b"default:diamond");
    Ok(())
}

#[async_std::test]
async fn test_change() -> Result<(), Box<dyn Error>> {
    common::tear_up().await?;
    // No early return here, so that tear down happens in every case
    let result = change_voxel().await;
    let cleanup_result = common::tear_down().await;
    result?;
    cleanup_result?;
    Ok(())
}
