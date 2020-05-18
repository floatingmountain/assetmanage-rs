use crate::{
    asset::{Asset, AssetHandle},
    loader::LoadStatus,
};
use crossbeam::{Receiver, Sender};
use slab::Slab;
use std::path::PathBuf;
use std::{collections::HashSet, error::Error, io::ErrorKind, sync::Arc};

/// Manages the loading and unloading of one struct that implements the Asset trait.
/// Regular calls to maintain support lazy loading, auto unload(optional default:off) and auto drop(optional default:off).
#[derive(Clone)]
pub struct Manager<A: Asset> {
    min_ref_drop: bool,
    min_ref_unload: bool,
    loader_id: usize,
    load_send: Sender<(usize, usize, PathBuf)>,
    load_recv: Receiver<(usize, Vec<u8>)>,
    asset_paths: HashSet<PathBuf>,
    asset_handles: Slab<AssetHandle<A>>,
}

impl<A: Asset> Manager<A> {
    /// Construct a new, empty `Manager`.
    ///
    /// The function does not allocate and the returned Managers main storage will have no
    /// capacity until `insert` is called.
    pub(crate) fn new(
        loader_id: usize,
        load_send: Sender<(usize, usize, PathBuf)>,
        load_recv: Receiver<(usize, Vec<u8>)>,
    ) -> Self {
        Self {
            min_ref_drop: false,
            min_ref_unload: false,
            loader_id,
            load_send,
            load_recv,
            asset_paths: HashSet::new(),
            asset_handles: Slab::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.asset_handles.capacity()
    }
    /// Set the `auto_dropout` of the Manager to `true`.
    ///
    /// The Manager will drop the AssetHandle on the next call of its `maintain` function
    /// if the asset is not loaded.
    ///
    /// After dropping the AssetHandle the `key` may be reused!
    ///
    pub fn auto_dropout(mut self) -> Self {
        self.min_ref_drop = true;
        self
    }
    /// Set the `auto_unload` of the Manager to `true`.
    ///
    /// The Manager will drop its reference to the Asset on the next call of its `maintain` function
    /// if its strong_refcount is equal to 1.
    ///
    pub fn auto_unload(mut self) -> Self {
        self.min_ref_unload = true;
        self
    }
    /// Insert an Assets Path into the Manager and return its key without loading the asset.
    /// If the specified path is already known to the Manager it will return the known paths key.
    ///
    /// If auto_dropout is activated the Asset has to be explicitly loaded with the given key after inserting
    /// or it will be dropped in the next call to maintain.
    ///
    pub fn insert(&mut self, path: PathBuf) -> usize {
        if self.asset_paths.contains(&path) {
            self.asset_handles
                .iter()
                .position(|(_, h)| h.path.eq(&path))
                .unwrap()
        } else {
            self.asset_paths.insert(path.clone());
            self.asset_handles.insert(AssetHandle::new(path))
        }
    }
    /// Insert an Assets Path and the loaded Asset into the Manager and return its key.
    /// If the specified path is already known to the Manager it will return the known paths key.
    ///
    /// If auto_dropout is activated the Asset has to be explicitly loaded with the given key after inserting
    /// or it will be dropped in the next call to maintain.
    ///
    pub fn insert_raw(&mut self, path: PathBuf, asset:A) -> usize {
        if self.asset_paths.contains(&path) {
            self.asset_handles
                .iter()
                .position(|(_, h)| h.path.eq(&path))
                .unwrap()
        } else {
            self.asset_paths.insert(path.clone());
            let mut handle = AssetHandle::new(path);
            handle.set(asset);
            self.asset_handles.insert(handle)
        }
    }
    ///// Loads an unloaded Asset known to the the Manager and returns its Arc<T>.
    ///// If the asset is already loaded it will just return the Asset.
    /////
    ///// If there is no valid file found at the specified path it will return an io::Error.
    ///// If the key is not found it will return None.
    /////
    //pub async fn load_blocking(&mut self, key: usize) -> Option<Result<Arc<A>, std::io::Error>> {
    //    Some(self.asset_handles.get_mut(key)?.load().await)
    //}
    /// Marks an unloaded Asset known to the the Manager as should_load.
    ///
    /// In next call to maintenance it will be attempted to load the Asset.
    ///
    pub fn load(&mut self, key: usize) -> Result<(), Box<dyn Error>> {
        let mut a = self.asset_handles.get_mut(key).ok_or(std::io::Error::new(
            ErrorKind::NotFound,
            format!("Key {} not found", key),
        ))?;
        a.status = LoadStatus::Loading;
        Ok(self.load_send.send((self.loader_id, key, a.path.clone()))?)
    }
    /// Unloads an Asset known to the the Manager. The Asset can be reloaded with the same key.
    ///
    /// The Arc of the Asset will be dropped. The Asset may still be used but the Manager wont know about it anymore.
    /// If the key is not found it will do nothing.
    ///
    pub fn unload(&mut self, key: usize) {
        if let Some(handle) = self.asset_handles.get_mut(key) {
            handle.unload()
        }
    }
    /// Drops an Asset known to the the Manager. The key may be reused by another Asset.
    ///
    /// If the key is not found it will do nothing.
    ///
    pub fn drop(&mut self, key: usize) {
        if let Some(handle) = self.asset_handles.get(key) {
            self.asset_paths.remove(&handle.path);
            self.asset_handles.remove(key);
        }
    }
    /// Returns an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    /// If the Asset is not loaded it will return None.
    /// Call status() to get detailed information.
    ///
    pub fn get(&self, key: usize) -> Option<Arc<A>> {
        Some(self.asset_handles.get(key)?.get()?.clone())
    }
    /// Returns an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    /// If the Asset is not loading it will return None.
    /// Will poll the loader until the asset is available and then returning it.
    ///
    pub fn get_blocking(&mut self, key: usize) -> Option<Arc<A>> {
        match self.asset_handles.get(key)?.get() {
            None => {
                if self.asset_handles.get(key)?.status.eq(&LoadStatus::Loading) {
                    while let Ok((k, b)) = self.load_recv.recv() {
                        if let Ok(a) = A::decode(&b) {
                            if let Some(handle) = self.asset_handles.get_mut(k) {
                                handle.set(a);
                                if key == k {
                                    return Some(handle.get()?.clone());
                                }
                            }
                        }
                    }
                }
                None
            }
            Some(a) => Some(a.clone()),
        }
    }
    /// Returns an Asset known to the the Manager.
    ///
    /// If a key is not found the Option will be None.
    /// If the Asset is not loading the Option will be None.
    /// Will poll the loader until all assets are available and then returning them.
    /// Order will be perserved.
    ///
    pub fn get_blocking_list(&mut self, keys: &[usize]) -> Vec<Option<Arc<A>>> {
        let mut assets = Vec::with_capacity(keys.len());
        for key in keys{
            assets.push(self.get_blocking(*key))
        }
        assets
    }
    /// Returns the LoadStatus of an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    ///
    pub fn status(&self, key: usize) -> Option<LoadStatus> {
        Some(self.asset_handles.get(key)?.status)
    }
    /// Maintains the manager. Needs to be called for lazy loading, to unload unused Assets and maybe even drop them.
    /// The default Manager will not drop or unload any Assets. So maintain will just load Assets.
    /// Will be slow if used with a large initial capacity + min_drop + min_unload as it will iterate over every Asset.
    ///
    pub fn maintain(&mut self) {
        if self.min_ref_drop || self.min_ref_unload {
            let mut keys_to_drop = Vec::new();
            for (key, handle) in self.asset_handles.iter_mut() {
                if handle.status == LoadStatus::Loaded {
                    if let Some(arc) = handle.get() {
                        if self.min_ref_unload && Arc::strong_count(&arc) == 1 {
                            handle.unload();
                        }
                    }
                }
                if self.min_ref_drop && handle.status != LoadStatus::Loading {
                    self.asset_paths.remove(&handle.path);
                    keys_to_drop.push(key);
                }
            }
            for key in keys_to_drop {
                self.drop(key);
            }
        }
        while let Ok((key, b)) = self.load_recv.try_recv() {
            if let Ok(a) = A::decode(&b) {
                if let Some(handle) = self.asset_handles.get_mut(key) {
                    handle.set(a)
                }
            }
        }
    }
    pub fn strong_count(&self, key: usize) -> Option<usize> {
        Some(Arc::strong_count(self.asset_handles.get(key)?.get()?))
    }
}

impl<A: Asset> Iterator for Manager<A> {
    type Item = Option<Arc<A>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.asset_handles
            .iter()
            .next()
            .map(|(_, a)| a.get().map(|a| a.clone()))
    }
}
