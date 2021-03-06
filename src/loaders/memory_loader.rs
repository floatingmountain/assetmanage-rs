use crate::{
    sources::{DiskSource, Source},
    Loader,
};
use futures::stream::{FuturesUnordered, StreamExt};
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};
///MemoryLoader recieves assets to load from the associated Managers, then loads and returns them asynchronous.
pub struct MemoryLoader {
    to_load: Receiver<(usize, PathBuf, <Self as Loader>::TransferSupplement)>,
    loaded: Vec<Sender<(PathBuf, Vec<u8>)>>,
}

impl super::Loader for MemoryLoader {
    type Source = DiskSource;
    type TransferSupplement = ();
    type LoaderSupplement = ();
    fn new(
        to_load: Receiver<(usize, PathBuf, Self::TransferSupplement)>,
        loaded: Vec<Sender<(PathBuf, <Self::Source as Source>::Output)>>,
        _: Self::LoaderSupplement,
    ) -> Self {
        Self { to_load, loaded }
    }
}

impl MemoryLoader {
    #[allow(unused)]
    pub(crate) fn new(
        to_load: Receiver<(usize, PathBuf, <Self as Loader>::TransferSupplement)>,
        loaded: Vec<Sender<(PathBuf, Vec<u8>)>>,
    ) -> Self {
        Self { to_load, loaded }
    }
    /// run the async load loop
    #[allow(unused)]
    pub async fn run(mut self) {
        let mut loading = FuturesUnordered::new();
        loop {
            self.to_load.try_iter().for_each(|(id, p, supp)| {
                loading.push(async move {
                    (id, p.clone(), <<Self as Loader>::Source as Source>::load(p))
                })
            });

            if let Some((manager_idx, path, Ok(bytes))) = loading.next().await {
                if let Some(sender) = self.loaded.get_mut(manager_idx) {
                    if sender.send((path, bytes)).is_err() {}
                }
            }
        }
    }
}
