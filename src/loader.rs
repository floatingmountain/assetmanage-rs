use std::path::{Path, PathBuf};
use futures::{
    stream::{FuturesUnordered, Stream, StreamExt},
    Future,
};
use slab::Slab;
use std::{
    any::Any,
};

use crossbeam::{Receiver, Sender, unbounded};
use crate::Asset;

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
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
    pub(crate) fn new(to_load: Receiver<(usize, usize, PathBuf)>, loaded: Slab<Sender<(usize,Vec<u8>)>>) -> Self { Self { to_load, loaded } }

    pub async fn run(self) {
        let (s,r) = unbounded();
        let loop_recv = || async {
            loop {
                if let Ok((key, f_key, path)) = self.to_load.try_recv() {
                    if s.try_send(load(key, f_key, path)).is_err(){
                        //info!("Loader Channel Closed");
                        break
                    }
                }
            }
        };
        let mut loaded = self.loaded.clone();

        let loop_load = || async move {
            let mut loading = FuturesUnordered::new();
            loop {
                while let Ok(fut) = r.try_recv(){
                    loading.push(fut);
                }
                if let Some((key, f_key, Ok(byt))) = loading.next().await {
                    if let Some(sender) = loaded.get_mut(key) {
                        if sender.send((f_key, byt)).is_err() {
                        }
                    }
                }
            }
        };
        futures::join!(loop_recv(), loop_load());
    }
}

async fn load<P: AsRef<Path>>(key: usize, f_key:usize, path: P) -> (usize, usize, Result<Vec<u8>,async_std::io::Error>) {
    (key, f_key, async_std::fs::read(path.as_ref()).await)
}
