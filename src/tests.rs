use super::*;
use serde::Deserialize;
use std::{time::Duration, io::ErrorKind};

use async_std::{path::Path, task};

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
    let path_to_testfile = std::env::current_dir()
        .unwrap()
        .join("assets/TestAsset.ron");
    let path_to_testfilecopy = std::env::current_dir()
        .unwrap()
        .join("assets/TestAssetCopy.ron");
    let mut builder = builder::Builder::new();
    //default manager
    let mut manager = builder.create_manager::<TestStruct>();
    //auto_unload_manager
    let mut manager2 = builder.create_manager::<TestStruct>().auto_unload(true);
    //auto_unload + auto_dropout manager
    let mut manager3 = builder.create_manager::<TestStruct>().auto_unload(true).auto_dropout(true);
    
    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());
    {
        //default demonstration
        let key = manager.insert(path_to_testfile.clone());
        let key_same = manager.insert(path_to_testfile.clone());
        assert!(key == key_same);
        let key_different = manager.insert(path_to_testfilecopy.clone());
        assert!(key != key_different); //Different Keys for different paths
        assert!(manager.get(key).is_none()); //Asset not loaded
        manager.load(key).unwrap();
        assert!(manager.get(key).is_none()); //Asset still not loaded
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager.maintain();
        manager.get(key).unwrap(); //Asset is loaded
        manager.unload(key);
        assert!(manager.get(key).is_none()); //Asset not loaded
    }
    {
        //auto-unload demonstration
        let key = manager2.insert(path_to_testfile.clone());
        assert!(manager2.get(key).is_none()); //Asset not loaded
        manager2.load(key);
        assert!(manager2.get(key).is_none()); //Asset not loaded
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager2.maintain();
        {
            let _val = manager2.get(key).unwrap(); //Asset is loaded
            manager2.maintain();
            assert!(manager2.get(key).is_some()); //Asset wont be unloaded while there is a cloned Arc used somewhere
        } // arc is dropped here
        manager2.maintain();
        assert!(manager2.get(key).is_none()); //Asset has been automatically unloaded
        manager2.load(key).unwrap(); //Asset can still be reloaded
    }
    {
        //auto-dropout + auto-unload demonstration
        let key = manager3.insert(path_to_testfile.clone());
        assert!(manager3.get(key).is_none()); //Asset not loaded
        manager3.maintain();
        assert!(manager3.load(key).is_err()); //It was instantly dropped during maintain. Cant be loaded
        let new_key = manager3.insert(path_to_testfile.clone());
        assert!(key == new_key); //dropped Key is being reused
        assert!(manager3.get(key).is_none()); //Asset not loaded
        manager3.load(new_key).unwrap();
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager3.maintain();
        {
            let _val = manager3.get(key).unwrap(); //Asset is loaded
        } // _val is dropped here
        manager3.maintain();
        assert!(manager3.load(key).is_err()); //Asset cant be reloaded, because the key has been dropped.
    }
}
