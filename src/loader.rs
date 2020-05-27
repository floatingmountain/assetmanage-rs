use futures::stream::{FuturesUnordered, StreamExt};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

///Loader recieves assets to load from the associated Managers, then loads and returns them asynchronous.
pub struct Loader {
    to_load: Receiver<(usize, PathBuf)>,
    loaded: Vec<Sender<(PathBuf, Vec<u8>)>>,
}

impl Loader {
    pub(crate) fn new(
        to_load: Receiver<(usize, PathBuf)>,
        loaded: Vec<Sender<(PathBuf, Vec<u8>)>>,
    ) -> Self {
        Self { to_load, loaded }
    }
    /// run the async load loop
    #[allow(unused)]
    pub async fn run(mut self) {
        let mut loading = FuturesUnordered::new();
        loop {
            while let Ok((manager_idx,path)) = self.to_load.try_recv() {
                loading.push(load(manager_idx,path));
            }
            if let Some(Ok((manager_idx, path, bytes))) = loading.next().await {
                if let Some(sender) = self.loaded.get_mut(manager_idx) {
                    if sender.send((path, bytes)).is_err() {}
                }
            }
        }
    }
}

// https://async.rs/blog/stop-worrying-about-blocking-the-new-async-std-runtime/
async fn load(manager_idx:usize, path: PathBuf) -> std::io::Result<(usize, PathBuf, Vec<u8>)> {
    let mut file = std::fs::File::open(&path)?;
    let mut contents = vec![];
    file.read_to_end(&mut contents)?;
    Ok((manager_idx, path, contents))
}
