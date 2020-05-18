use legion::prelude::*;
use assetmanage_rs::*;
use serde::Deserialize;
use std::{io::ErrorKind};


extern crate pretty_env_logger;
#[macro_use] extern crate log;

/// TestStruct demonstrates implementing Asset
#[derive(Deserialize,Debug)]
struct TestStruct {
    _s: String,
}

impl Asset for TestStruct {
    fn decode(b: &[u8]) -> Result<Self, std::io::Error>{
        
        ron::de::from_bytes::<TestStruct>(&b)
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
    }
}

fn main() {
    pretty_env_logger::formatted_builder()
        //let's just set some random stuff.. for more see
        //https://docs.rs/env_logger/0.5.0-rc.1/env_logger/struct.Builder.html
        .parse_filters("with_builder_1=trace")
        .init();
    // create world
    let universe = Universe::new();
    let mut world = universe.create_world();

    let mut builder = assetmanage_rs::Builder::new();

    world.resources.insert(builder.create_manager::<TestStruct>());
    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());
    
    
    let maintain_assets = SystemBuilder::new("maintain_assets")
    .write_resource::<Manager<TestStruct>>()
    .build(|_,_,manager_test_struct,_|{
        info!("maintaining");
        manager_test_struct.maintain();
    });

    let load_asset = SystemBuilder::new("load_asset")
    .write_resource::<Manager<TestStruct>>()
    .build(|_,_,manager_test_struct,_|{
        info!("loading");
        let path_to_testfile = std::env::current_dir()
        .unwrap()
        .join("assets/TestAsset.ron");
        let key = manager_test_struct.insert(path_to_testfile);
        manager_test_struct.load(key).expect("FML");
    });

    let getasset = SystemBuilder::new("getasset")
    .write_resource::<Manager<TestStruct>>()
    .build(|_,_,manager_test_struct,_|{
        info!("{:?}",manager_test_struct.get(0));
    });

    let mut schedule = Schedule::builder()
        .add_system(maintain_assets)
        .add_system(load_asset)
        .add_system(getasset)
        .flush()
        .build();
    
    loop{schedule.execute(&mut world);}
}
