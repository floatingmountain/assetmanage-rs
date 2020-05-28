mod memory_loader;
pub use memory_loader::MemoryLoader;
use std::{path::PathBuf, sync::mpsc:: Sender};
use futures::channel::mpsc::UnboundedReceiver;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

pub trait Loader{
    type Return;
    fn new(
        to_load: UnboundedReceiver<(usize, PathBuf)>,
        loaded: Vec<Sender<(PathBuf, Self::Return)>>,
    ) -> Self;
}