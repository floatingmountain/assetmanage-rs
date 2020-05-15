use futures::{
    stream::{FuturesUnordered, Stream, StreamExt},
    Future,
};
use slab::Slab;
use std::any::Any;
use std::path::{Path, PathBuf};

use crate::Asset;
use crossbeam::{unbounded, Receiver, Sender};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

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

    pub async fn run(mut self) {
        loop {
            let mut loading = FuturesUnordered::new();

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
