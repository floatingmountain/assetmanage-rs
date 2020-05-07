use legion::prelude::*;
use assetmanage_rs::*;
use serde::Deserialize;
use std::{io::ErrorKind};
use async_trait::async_trait;
use async_std::{task, path::Path};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Pos(f32, f32, f32);

/// TestStruct demonstrates implementing Asset
#[derive(Deserialize)]
struct TestStruct {
    _s: String,
}

#[async_trait]
impl Asset for TestStruct {
     async fn load<P: AsRef<Path> + Send>(path: P) -> Result<Self, std::io::Error> {
        let b = async_std::fs::read(path).await?;
        ron::de::from_bytes::<TestStruct>(&b)
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
    }
}

fn main() {
    // create world
    let universe = Universe::new();
    let mut world = universe.create_world();

    
    let mut resources = Resources::default();
    resources.insert(Manager::<TestStruct>::new());


    let maintain_assets = SystemBuilder::new("maintain_assets")
    .write_resource::<Manager<TestStruct>>()
    .build(|_,_,manager_test_struct,_|{
        task::block_on( batch_maintain(
            vec![
                manager_test_struct
            ]
        ));
    });
    let load_asset = SystemBuilder::new("load_asset")
    .write_resource::<Manager<TestStruct>>()
    .build(|_,_,manager_test_struct,_|{
        let path_to_testfile = std::env::current_dir()
        .unwrap()
        .join("assets/TestAsset.ron");
        let key = manager_test_struct.insert(path_to_testfile);
        manager_test_struct.load_lazy(key);
    });


    let mut schedule = Schedule::builder()
        .add_system(maintain_assets)
        .build();
    
    schedule.execute(&mut world);
}
