use super::*;
use serde::Deserialize;
use std::io::ErrorKind;

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
    let mut manager = builder.create_manager::<TestStruct>();
    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());
    {
        let key = manager.insert(path_to_testfile.clone());
        let key_same = manager.insert(path_to_testfile.clone());
        assert!(key == key_same);
        let key_different = manager.insert(path_to_testfilecopy.clone());
        assert!(key != key_different); //Different Keys for different paths
        assert!(manager.get(key).is_none()); //Asset not loaded
        manager.load(key).unwrap();
        assert!(manager.get(key).is_none()); //Asset probably still not loaded
        std::thread::sleep_ms(500);
        manager.get(key).unwrap(); //Asset is loaded
        manager.unload(key);
        assert!(manager.get(key).is_none()); //Asset not loaded
    }
    //    {
    //        //default demonstration
    //        let mut manager: Manager<TestStruct> = Manager::new();
    //        let key = manager.insert(path_to_testfile.clone());
    //        let key_same = manager.insert(path_to_testfile.clone());
    //        assert!(key == key_same); //Same asset will get the same key
    //        let key_different = manager.insert(path_to_testfilecopy.clone());
    //        assert!(key != key_different); //Different Keys for different paths
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        manager.load_lazy(key);
    //        assert!(manager.get(key).is_none()); //Asset still not loaded
    //        task::block_on( manager.maintain());
    //        manager.get(key).unwrap(); //Asset is loaded
    //        manager.unload(key);
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        task::block_on( manager.load(key)).unwrap().unwrap(); //Asset is loaded
    //    }
    //    {
    //        //auto-unload demonstration
    //        let mut manager: Manager<TestStruct> = Manager::new().auto_unload(true);
    //        let key = manager.insert(path_to_testfile.clone());
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        manager.load_lazy(key);
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        task::block_on( manager.maintain());
    //        {
    //            let _val = manager.get(key).unwrap(); //Asset is loaded
    //            task::block_on( manager.maintain());
    //            assert!(manager.get(key).is_some()); //Asset wont be unloaded while there is a cloned Arc used somewhere
    //        } // arc is dropped here
    //        task::block_on( manager.maintain());
    //        assert!(manager.get(key).is_none()); //Asset has been automatically unloaded
    //        task::block_on( manager.load(key)).unwrap().unwrap(); //Asset can still be reloaded
    //    }
    //    {
    //        //auto-dropout + auto-unload demonstration
    //        let mut manager: Manager<TestStruct> = Manager::new().auto_unload(true).auto_dropout(true);
    //        let key = manager.insert(path_to_testfile.clone());
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        task::block_on( manager.maintain());
    //        assert!(task::block_on( manager.load(key)).is_none()); //Key not found -> It was dropped during maintain
    //        let new_key = manager.insert(path_to_testfile.clone());
    //        assert!(key == new_key); //dropped Key is being reused
    //        assert!(manager.get(key).is_none()); //Asset not loaded
    //        manager.load_lazy(new_key);
    //        task::block_on( manager.maintain());
    //        {
    //            let _val = manager.get(key).unwrap(); //Asset is loaded
    //        } // _val is dropped here
    //        task::block_on( manager.maintain());
    //        assert!(task::block_on( manager.load(key)).is_none()); //Asset cant be reloaded, because the key has been dropped.
    //    }
}
