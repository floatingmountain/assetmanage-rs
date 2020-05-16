use crate::{Asset, Manager, loader::Loader};
use std::path::PathBuf;
use crossbeam::{unbounded, Receiver, Sender};
use slab::Slab;

#[allow(unused)]
pub struct Builder {
    to_load_send: Sender<(usize, usize, PathBuf)>,
    to_load_recv: Receiver<(usize, usize, PathBuf)>,
    loaded: Slab<Sender<(usize, Vec<u8>)>>,
}

impl Builder {
    #[allow(unused)]
    pub fn new() -> Self {
        let (to_load_send, to_load_recv) = unbounded();
        Self {
            to_load_send,
            to_load_recv,
            loaded: Slab::new(),
        }
    }
    #[allow(unused)]
    pub fn create_manager<A: Asset>(&mut self) -> Manager<A> {
        let (s, r) = unbounded();
        let loader_id = self.loaded.insert(s);
        Manager::new(loader_id, self.to_load_send.clone(), r)
    }
    #[allow(unused)]
    pub fn finish_loader(self) -> Loader{
        Loader::new(self.to_load_recv, self.loaded)
    }
}
