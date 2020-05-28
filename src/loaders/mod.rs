mod memory_loader;
pub use memory_loader::MemoryLoader;
use std::{path::PathBuf, sync::mpsc:: {Sender,Receiver}};
use crate::sources::Source;
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

pub trait Loader{
    type Output: Source;
    fn new(
        to_load: Receiver<(usize, PathBuf)>,
        loaded: Vec<Sender<(PathBuf, <Self::Output as Source>::Output)>>,
    ) -> Self;
}