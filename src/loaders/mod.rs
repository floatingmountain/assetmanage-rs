mod memory_loader;
use crate::sources::Source;
pub use memory_loader::MemoryLoader;
use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender},
};
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub enum LoadStatus {
    NotLoaded,
    Loading,
    Loaded,
}

pub trait Loader {
    type Source: Source;
    type TransferSupplement: Send + Sync;
    type LoaderSupplement;
    fn new(
        to_load: Receiver<(usize, PathBuf, Self::TransferSupplement)>,
        loaded: Vec<Sender<(PathBuf, <Self::Source as Source>::Output)>>,
        data: Self::LoaderSupplement,
    ) -> Self;
}
