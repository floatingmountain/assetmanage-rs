use crate::{
    asset::{Asset, AssetHandle},
    loaders::{LoadStatus, Loader},
    sources::Source,
};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::{collections::HashMap, error::Error, io::ErrorKind, sync::Arc};

/// Manages the loading and unloading of one struct that implements the Asset trait.
/// Regular calls to maintain support lazy loading, auto unload(optional default:off) and auto drop(optional default:off).
pub struct Manager<A, L>
where
    A: Asset<L>,
    L: Loader,
{
    drop: bool,
    unload: bool,
    loader_id: usize,
    load_send: Sender<(usize, PathBuf)>,
    load_recv: Receiver<(PathBuf, <L::Source as Source>::Output)>,
    asset_handles: HashMap<PathBuf, AssetHandle<A, L>>,
    loaded_once: Vec<PathBuf>,
    data: A::DataManager,
}

unsafe impl<A, L> Sync for Manager<A, L>
where
    A: Asset<L>,
    L: Loader,
{
} //channels are unsafe to send but are only used internally.

impl<A, L> Manager<A, L>
where
    A: Asset<L>,
    L: Loader,
{
    /// Construct a new, empty `Manager`.
    ///
    /// The function does not allocate and the returned Managers main storage will have no
    /// capacity until `insert` is called.
    pub(crate) fn new(
        loader_id: usize,
        load_send: Sender<(usize, PathBuf)>,
        load_recv: Receiver<(PathBuf, <L::Source as Source>::Output)>,
        data: A::DataManager,
    ) -> Self {
        Self {
            drop: false,
            unload: false,
            loader_id,
            load_send,
            load_recv,
            asset_handles: HashMap::new(),
            loaded_once: Vec::new(),
            data,
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
        self.drop = true;
        self
    }
    /// Set the `auto_unload` of the Manager to `true`.
    ///
    /// The Manager will drop its reference to the Asset on the next call of its `maintain` function
    /// if its strong_refcount is equal to 1.
    ///
    pub fn auto_unload(mut self) -> Self {
        self.unload = true;
        self
    }
    /// Insert an Assets Path into the Manager and return its key without loading the asset.
    /// If the specified path is already known to the Manager it will return the known paths key.
    ///
    /// If auto_dropout is activated the Asset has to be explicitly loaded with the given key after inserting
    /// or it will be dropped in the next call to maintain.
    ///
    pub fn insert<P: AsRef<Path>>(&mut self, path: P, data: A::DataAsset) {
        let path: PathBuf = path.as_ref().into();
        self.asset_handles
            .entry(path.clone())
            .or_insert(AssetHandle::new(path, data));
    }
    /// Insert an Assets Path and the loaded Asset into the Manager and return its key.
    /// If the specified path is already known to the Manager it will return the known paths key.
    ///
    /// If auto_dropout is activated the Asset has to be explicitly loaded with the given key after inserting
    /// or it will be dropped in the next call to maintain.
    ///
    pub fn insert_raw<P: AsRef<Path>>(&mut self, path: P, asset: A::Output, data: A::DataAsset) {
        let path: PathBuf = path.as_ref().into();
        let mut handle = AssetHandle::new(path.clone(), data);
        handle.set(asset);
        self.asset_handles.insert(path, handle);
    }
    /// Loads an unloaded Asset known to the the Manager and returns its Arc<T>.
    /// If the asset is already loaded it will just return the Asset.
    ///
    /// If there is no valid file found at the specified path it will return an io::Error.
    /// If the key is not found it will return None.
    ///
    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Box<dyn Error>> {
        let mut a = self
            .asset_handles
            .get_mut(path.as_ref())
            .ok_or(std::io::Error::new(
                ErrorKind::NotFound,
                format!("Entry not found! {:?}", path.as_ref()),
            ))?;
        if !path.as_ref().exists() {
            return Err(Box::new(std::io::Error::new(
                ErrorKind::NotFound,
                format!("File not found! {:?}", path.as_ref()),
            )));
        }
        a.status = LoadStatus::Loading;
        Ok(self
            .load_send
            .send((self.loader_id, path.as_ref().into()))?)
    }
    /// Unloads an Asset known to the the Manager. The Asset can be reloaded with the same key.
    ///
    /// The Arc of the Asset will be dropped. The Asset may still be used but the Manager wont know about it anymore.
    /// If the key is not found it will do nothing.
    ///
    pub fn unload<P: AsRef<Path>>(&mut self, path: P) {
        if let Some(handle) = self.asset_handles.get_mut(path.as_ref()) {
            handle.unload()
        }
    }
    /// Drops an Asset known to the the Manager. The key may be reused by another Asset.
    ///
    /// If the key is not found it will do nothing.
    ///
    pub fn drop<P: AsRef<Path>>(&mut self, path: P) {
        self.asset_handles.remove(path.as_ref());
    }
    /// Returns an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    /// If the Asset is not loaded it will return None.
    /// Call status() to get detailed information.
    ///
    pub fn get<P: AsRef<Path>>(&self, path: P) -> Option<Arc<A::Output>> {
        Some(self.asset_handles.get(path.as_ref())?.get()?.clone())
    }
    /// Returns an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    /// If the Asset is not loading it will return None.
    /// Will wait for the Asset to become available on the receiver and then returning it.
    ///
    pub fn get_blocking<P: AsRef<Path>>(&mut self, path: P) -> Option<Arc<A::Output>> {
        match self.asset_handles.get(path.as_ref())?.get() {
            None => {
                if let Some(handle) = self.asset_handles.get_mut(path.as_ref()) {
                    if handle.status.eq(&LoadStatus::Loading) {
                        while let Ok((p, out)) = self.load_recv.recv() {
                            if let Ok(a) = A::decode(out, &handle.data, &self.data) {
                                handle.set(a);
                                self.loaded_once.push(path.as_ref().into());
                                if p.eq(path.as_ref()) {
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
    /// Returns loaded assets once as soon as they have the LoadStatus::Loaded.
    pub fn get_loaded_once(&mut self) -> Vec<PathBuf> {
        let mut list = Vec::new();
        if !self.loaded_once.is_empty() {
            std::mem::swap(&mut list, &mut self.loaded_once);
        }
        list
    }
    /// Returns the LoadStatus of an Asset known to the the Manager.
    ///
    /// If the key is not found it will return None.
    ///
    pub fn status<P: AsRef<Path>>(&mut self, path: P) -> Option<LoadStatus> {
        Some(self.asset_handles.get(path.as_ref())?.status)
    }
    /// Maintains the manager. Needs to be called for lazy loading, to unload unused Assets and maybe even drop them.
    /// The default Manager will not drop or unload any Assets. So maintain will just load Assets.
    /// Will be slow if used with a large initial capacity + min_drop + min_unload as it will iterate over every Asset.
    ///
    pub fn maintain(&mut self) {
        if self.unload {
            self.asset_handles
                .values_mut()
                .filter(|h| h.status.eq(&LoadStatus::Loaded))
                .filter(|h| Arc::strong_count(h.get().unwrap()).eq(&1))
                .for_each(|h| h.unload());
        }
        if self.drop {
            let mut paths_to_drop = Vec::new();
            for (path, handle) in self.asset_handles.iter() {
                if self.drop && handle.status != LoadStatus::Loading {
                    paths_to_drop.push(path.clone());
                }
            }
            for path in paths_to_drop {
                self.drop(path);
            }
        }
        for (p, b) in self.load_recv.try_iter() {
            if let Some(handle) = self.asset_handles.get_mut(p.as_path()) {
                if let Ok(a) = A::decode(b, &handle.data, &self.data) {
                    handle.set(a);
                    self.loaded_once.push(p);
                }
            }
        }
    }
    pub fn strong_count<P: AsRef<Path>>(&mut self, path: P) -> Option<usize> {
        Some(Arc::strong_count(
            self.asset_handles.get(path.as_ref())?.get()?,
        ))
    }
}

impl<A, L> Iterator for Manager<A, L>
where
    A: Asset<L>,
    L: Loader,
{
    type Item = Option<Arc<A::Output>>;
    fn next(&mut self) -> Option<Self::Item> {
        self.asset_handles
            .iter()
            .next()
            .map(|(_, a)| a.get().map(|a| a.clone()))
    }
}
