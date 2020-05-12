use crate::{Asset, Manager, loader::Loader};
use std::path::PathBuf;
use crossbeam::{unbounded, Receiver, Sender};
use slab::Slab;
pub struct Builder {
    to_load_send: Sender<(usize, PathBuf)>,
    to_load_recv: Receiver<(usize, PathBuf)>,
    loaded: Slab<Sender<Vec<u8>>>,
}

impl Builder {
    pub fn new() -> Self {
        let (to_load_send, to_load_recv) = unbounded();
        Self {
            to_load_send,
            to_load_recv,
            loaded: Slab::new(),
        }
    }

    pub fn create_manager<A: Asset>(&mut self) -> Manager<A> {
        let (s, r) = unbounded();
        let loader_id = self.loaded.insert(s);
        Manager::new(loader_id, self.to_load_send.clone(), r)
    }

    pub fn finish_loader(self) -> Loader{
        Loader::new(self.to_load_recv, self.loaded)
    }
}
