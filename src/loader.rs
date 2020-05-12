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

pub enum LoadStatus {
    NotLoading,
    Loading,
    Loaded,
}


pub struct Loader {
    to_load: Receiver<(usize, PathBuf)>,
    loaded: Slab<Sender<Vec<u8>>>,
}

impl Loader {
    pub(crate) fn new(to_load: Receiver<(usize, PathBuf)>, loaded: Slab<Sender<Vec<u8>>>) -> Self { Self { to_load, loaded } }

    async fn run(&mut self) {
        let (s,r) = unbounded();
        let loop_recv = || async {
            loop {
                if let Ok((key, path)) = self.to_load.try_recv() {
                    if s.try_send(load(key, path)).is_err(){
                        //info!("Loader Channel Closed");
                        break
                    }
                }
            }
        };
        let mut loaded = self.loaded.clone();

        let loop_load = || async move {
            let mut loading = FuturesUnordered::new();
            let mut drop_some: Option<usize> = None;
            loop {
                while let Ok(fut) = r.try_recv(){
                    loading.push(fut);
                }
                if let Some((key, byt)) = loading.next().await {
                    if let Some(sender) = loaded.get_mut(key) {
                        if sender.send(byt).is_err() {
                            drop_some = Some(key);
                        }
                    }
                }
                if let Some(key) = drop_some {
                    loaded.remove(key);
                    drop_some = None;
                }
            }
        };
        futures::join!(loop_recv(), loop_load());
    }
}

async fn load<P: AsRef<Path>>(key: usize, path: P) -> (usize, Vec<u8>) {
    todo!()
}
