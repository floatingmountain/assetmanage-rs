use super::*;
use serde::Deserialize;
use std::{time::Duration, io::ErrorKind};
use loader::LoadStatus;

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
    let mut manager1 = builder.create_manager::<TestStruct>();
    //auto_unload_manager
    let mut manager2 = builder.create_manager::<TestStruct>().auto_unload(true);
    //auto_dropout manager
    let mut manager3 = builder.create_manager::<TestStruct>().auto_dropout(true);
    //auto_unload + auto_dropout manager
    let mut manager4 = builder.create_manager::<TestStruct>().auto_unload(true).auto_dropout(true);
    
    let loader = builder.finish_loader();
    async_std::task::spawn(loader.run());
    {
        //default demonstration
        let key = manager1.insert(path_to_testfile.clone());
        let key_same = manager1.insert(path_to_testfile.clone());
        assert!(key == key_same);
        let key_different = manager1.insert(path_to_testfilecopy.clone());
        assert!(key != key_different); //Different Keys for different paths
        assert!(manager1.status(key).eq(&Some(LoadStatus::NotLoaded))); //Asset not loaded
        manager1.load(key).unwrap();
        assert!(manager1.status(key).eq(&Some(LoadStatus::Loading))); //Asset still not loaded but is loading
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager1.maintain(); //Asset is fetched from Loader during maintain
        assert!(manager1.status(key).eq(&Some(LoadStatus::Loaded))); //Asset is loaded
        let _a = manager1.get(key).unwrap(); //Get the loaded asset 
        manager1.unload(key); // manually unload the asset
        assert!(manager1.status(key).eq(&Some(LoadStatus::NotLoaded))); //Asset not loaded
        assert!(manager1.get(key).is_none()); //Cannot get the asset
        drop(manager1);
    }
    {
        //auto-unload demonstration
        let key = manager2.insert(path_to_testfile.clone());
        manager2.load(key).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        manager2.maintain();
        {
            let _val = manager2.get(key).unwrap(); //Asset is loaded
            manager2.maintain(); //Asset wont be unloaded during this maintain
            assert!(manager2.status(key).eq(&Some(LoadStatus::Loaded))); //Asset wont be unloaded while there is a cloned Arc used somewhere
        } // arc is dropped here
        manager2.maintain(); //Asset will be dropped during this maintain
        assert!(manager2.status(key).eq(&Some(LoadStatus::NotLoaded))); //Asset has been automatically unloaded
        assert!(manager2.get(key).is_none()); //Asset cannot be retrieved
        manager2.load(key).unwrap(); //Asset can still be reloaded with the same key
        drop(manager2);
    }
    {
        //auto-dropout demonstration
        let key = manager3.insert(path_to_testfile.clone());
        manager3.maintain(); //Assethandle will be dropped during this maintain. Cant be loaded afterwards.
        assert!(manager3.load(key).is_err()); //Cant be loaded
        let new_key = manager3.insert(path_to_testfile.clone()); //reinsert
        assert!(key == new_key); //dropped Key is being reused
        assert!(manager3.status(key).eq(&Some(LoadStatus::NotLoaded))); //Asset not loaded
        manager3.load(new_key).unwrap();
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        assert!(manager3.status(key).eq(&Some(LoadStatus::Loading))); //Asset wont be dropped while it is loading
        manager3.maintain(); // Asset will be loaded during this maintain
        manager3.get(key).unwrap(); //Asset is loaded
        manager3.unload(key); // manually unload the asset
        manager3.maintain(); // asset will now be dropped because it is unloaded
        assert!(manager3.status(key).eq(&None)); //Asset not loaded
        drop(manager3);
    }
    {
        //auto-dropout + auto-unload demonstration
        let key = manager4.insert(path_to_testfile.clone());
        manager4.load(key).unwrap();
        std::thread::sleep(Duration::from_millis(50)); //wait for load
        manager4.maintain();
        {
            let _val = manager4.get(key).unwrap(); //Asset is loaded
        } // _val is dropped here
        manager4.maintain(); //Asset is dropped here because noone holds a ref.
        assert!(manager4.load(key).is_err()); //Asset cant be reloaded, because the key has been dropped when there was no remaining ref to it.
        drop(manager4);
    }
}
