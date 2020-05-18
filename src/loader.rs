use futures::{
    stream::{FuturesUnordered, StreamExt},
    };
use slab::Slab;
use std::path::{Path, PathBuf};

use crossbeam::{ Receiver, Sender};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}
///Loader recieves assets to load from the associated Managers, then loads and returns them asynchronous.
pub struct Loader {
    to_load: Receiver<(usize, usize, PathBuf)>,
    loaded: Slab<Sender<(usize, Vec<u8>)>>,
}

impl Loader {
    pub(crate) fn new(
        to_load: Receiver<(usize, usize, PathBuf)>,
        loaded: Slab<Sender<(usize, Vec<u8>)>>,
    ) -> Self {
        Self { to_load, loaded }
    }
    /// run the async load loop
    ///    
    ///
    /// # Example
    ///
    /// ```
    /// let mut builder = builder::Builder::new();
    /// [...]
    /// let loader = builder.finish_loader();
    /// async_std::task::spawn(loader.run());
    /// ```
    #[allow(unused)]
    pub async fn run(mut self) {
        let mut loading = FuturesUnordered::new();
        loop {
            while let Ok((key, f_key, path)) = self.to_load.try_recv() {
                loading.push(load(key, f_key, path));
            }
            if let Some((key, f_key, Ok(byt))) = loading.next().await {
                if let Some(sender) = self.loaded.get_mut(key) {
                    if sender.send((f_key, byt)).is_err() {}
                }
            }
        }
    }
}

async fn load<P: AsRef<Path>>(
    key: usize,
    f_key: usize,
    path: P,
) -> (usize, usize, Result<Vec<u8>, async_std::io::Error>) {
    (key, f_key, async_std::fs::read(path.as_ref()).await)
}
