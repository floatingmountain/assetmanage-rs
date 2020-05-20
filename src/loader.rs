use futures::stream::{FuturesUnordered, StreamExt};
use slab::Slab;
use std::path::PathBuf;

use std::sync::mpsc::{Receiver,Sender};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

pub(crate) struct LoadPacket{
    manager_idx: usize,
    asset_key: usize,
    asset_path: PathBuf,
}

impl LoadPacket {
    pub(crate) fn new(manager_idx: usize, asset_key: usize, asset_path: PathBuf) -> Self { Self { manager_idx, asset_key, asset_path } }
}



///Loader recieves assets to load from the associated Managers, then loads and returns them asynchronous.
pub struct Loader {
    to_load: Receiver<LoadPacket>,
    loaded: Slab<Sender<(usize, Vec<u8>)>>,
}

impl Loader {
    pub(crate) fn new(
        to_load: Receiver<LoadPacket>,
        loaded: Slab<Sender<(usize, Vec<u8>)>>,
    ) -> Self {
        Self { to_load, loaded }
    }
    /// run the async load loop
    #[allow(unused)]
    pub async fn run(mut self) {
        let mut loading = FuturesUnordered::new();
        loop {
            while let Ok(packet) = self.to_load.try_recv(){
                loading.push(load(packet));
            }
            if let Some((manager_idx, asset_key, Ok(bytes))) = loading.next().await {
                if let Some(sender) = self.loaded.get_mut(manager_idx) {
                    if sender.send((asset_key, bytes)).is_err() {}
                }
            }
        }
    }
}

async fn load(
    packet: LoadPacket
) -> (usize, usize, Result<Vec<u8>, async_std::io::Error>) {
    (packet.manager_idx, packet.asset_key, async_std::fs::read(packet.asset_path).await)
}
