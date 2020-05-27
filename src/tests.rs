use super::*;
use loader::LoadStatus;
use serde::Deserialize;
use std::{io::ErrorKind, time::Duration};

/// TestStruct demonstrates implementing Asset
#[derive(Deserialize)]
struct TestStruct {
    _s: String,
}

impl Asset for TestStruct {
    fn decode(b: &[u8]) -> Result<Self, std::io::Error> {
        ron::de::from_bytes::<TestStruct>(&b)
            .map_err(|e| std::io::Error::new(ErrorKind::InvalidData, e))
    }
}

#[test]
///Demonstrates and tests the use of Manager
fn it_works() {
    let path1 = std::env::current_dir()
        .unwrap()
        .join("assets/TestAsset.ron");
    let path_to_testfilecopy = std::env::current_dir()
        .unwrap()
        .join("assets/TestAssetCopy.ron");
    let mut builder = builder::Builder::new();

    //default manager
    let mut manager1 = builder.create_manager::<TestStruct>();
    //auto_unload_manager
    let mut manager2 = builder.create_manager::<TestStruct>().auto_unload();
    //auto_dropout manager
    let mut manager3 = builder.create_manager::<TestStruct>().auto_dropout();
    //auto_unload + auto_dropout manager
    let mut manager4 = builder
        .create_manager::<TestStruct>()
        .auto_unload()
        .auto_dropout();

    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());
    {
        //default demonstration
        manager1.insert(&path1);
        assert!(manager1.status(&path1).eq(&Some(LoadStatus::NotLoaded))); //Asset not loaded
        manager1.load(&path1).unwrap();
        assert!(manager1.status(&path1).eq(&Some(LoadStatus::Loading))); //Asset still not loaded but is loading
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager1.maintain(); //Asset is fetched from Loader during maintain
        assert!(manager1.status(&path1).eq(&Some(LoadStatus::Loaded))); //Asset is loaded
        let _a = manager1.get(&path1).unwrap(); //Get the loaded asset
        manager1.unload(&path1); // manually unload the asset
        assert!(manager1.status(&path1).eq(&Some(LoadStatus::NotLoaded))); //Asset not loaded
        assert!(manager1.get(&path1).is_none()); //Cannot get the asset
        drop(manager1);
    }
    {
        //auto-unload demonstration
        manager2.insert(path1.clone());
        manager2.load(&path1).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        manager2.maintain();
        {
            let _val = manager2.get(&path1).unwrap(); //Asset is loaded
            manager2.maintain(); //Asset wont be unloaded during this maintain
            assert!(manager2.status(&path1).eq(&Some(LoadStatus::Loaded))); //Asset wont be unloaded while there is a cloned Arc used somewhere
        } // arc is dropped here
        manager2.maintain(); //Asset will be dropped during this maintain
        assert!(manager2.status(&path1).eq(&Some(LoadStatus::NotLoaded))); //Asset has been automatically unloaded
        assert!(manager2.get(&path1).is_none()); //Asset cannot be retrieved
        manager2.load(&path1).unwrap(); //Asset can still be reloaded with the same key
        drop(manager2);
    }
    {
        //auto-dropout demonstration
        manager3.insert(&path1);
        manager3.maintain(); //Assethandle will be dropped during this maintain. Cant be loaded afterwards.
        assert!(manager3.load(&path1).is_err()); //Cant be loaded
        assert!(manager3.status(&path1).eq(&None)); //Asset not loaded
        manager3.insert(&path1);
        manager3.load(&path1).unwrap();
        manager3.maintain();
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        assert!(manager3.status(&path1).eq(&Some(LoadStatus::Loading))); //Asset wont be dropped while it is loading
        manager3.maintain(); // Asset will be loaded during this maintain
        manager3.get(&path1).unwrap(); //Asset is loaded
        manager3.unload(&path1); // manually unload the asset
        manager3.maintain(); // asset will now be dropped because it is unloaded
        assert!(manager3.status(&path1).eq(&None)); //Asset not loaded
        drop(manager3);
    }
    {
        //auto-dropout + auto-unload demonstration
        manager4.insert(&path1);
        manager4.load(&path1).unwrap();
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager4.maintain();
        {
            let _val = manager4.get(&path1).unwrap(); //Asset is loaded
        } // _val is dropped here
        manager4.maintain(); //Asset is dropped here because noone holds a ref.
        assert!(manager4.load(&path1).is_err()); //Asset cant be reloaded, because the key has been dropped when there was no remaining ref to it.
        drop(manager4);
    }
}

#[test]
fn test_load_get() {
    let path = std::env::current_dir()
        .unwrap()
        .join("assets/TestAsset.ron");
    let path2 = std::env::current_dir()
        .unwrap()
        .join("assets/TestAssetCopy.ron");
    let mut builder = builder::Builder::new();
    let mut manager = builder.create_manager::<TestStruct>();
    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());

    manager.insert(&path);
    manager.insert(&path2);
    manager.load(&path).unwrap();
    manager.load(&path2).unwrap();
    let s1 = manager.get_blocking(&path).unwrap();
    let s2 = manager.get_blocking(&path2).unwrap();
    assert!(s1._s.eq(&String::from("12341234")));
    assert!(s2._s.eq(&String::from("123412345")));
    let s2 = manager.get_blocking(&path2).unwrap();
    let s1 = manager.get_blocking(&path).unwrap();
    assert!(s1._s.eq(&String::from("12341234")));
    assert!(s2._s.eq(&String::from("123412345")));
    let s2 = manager.get(&path2).unwrap();
    let s1 = manager.get(&path).unwrap();
    assert!(s1._s.eq(&String::from("12341234")));
    assert!(s2._s.eq(&String::from("123412345")));
}
