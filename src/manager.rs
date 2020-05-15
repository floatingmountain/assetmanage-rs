use crate::{
    asset::{Asset, AssetHandle},
    loader::LoadStatus,
};
use crossbeam::{Receiver, SendError, Sender};
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

    pub fn capacity(mut self) -> usize {
        self.asset_handles.capacity()
    }
    /// Set the `min_ref_drop` of the Manager.
    ///
    /// The Manager will drop the AssetHandle on the next call of its `maintain` function
    /// if the asset is not loaded.
    ///
    /// After dropping the AssetHandle the `key` may be reused!
    ///     
    /// # Arguments
    ///
    /// * `value = false` - (Default) wont ever drop the Handle
    /// * `value = true`  - will drop the handle when value equal strong_refcount
    ///
    pub fn auto_dropout(mut self, value: bool) -> Self {
        self.min_ref_drop = value;
        self
    }
    /// Set the `min_ref_unload` of the Manager.
    ///
    /// The Manager will drop its reference to the Asset on the next call of its `maintain` function
    /// if the strong_refcount is equal to the specified `min_ref_unload`.
    ///
    /// # Arguments
    ///
    /// * `value = false` - (Default) will not drop the reference to the Asset
    /// * `value = true`  - will drop the  the reference to the Asset when value equal strong_refcount
    ///
    pub fn auto_unload(mut self, value: bool) -> Self {
        self.min_ref_unload = value;
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
    ///
    pub fn get(&self, key: usize) -> Option<Arc<A>> {
        Some(self.asset_handles.get(key)?.get()?.clone())
    }
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
