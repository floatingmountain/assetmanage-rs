use crate::sources::Source;
use crate::{loaders::Loader, Asset, Manager};
use std::{
    marker::PhantomData,
    path::PathBuf,
    sync::mpsc::{channel, Receiver, Sender},
};
/// Builder is used to Build Managers with a loading backend.
/// construct a Builder, create Managers and finish by returning a loader.
#[allow(unused)]
pub struct Builder<L>
where
    L: Loader,
{
    to_load_send: Sender<(usize, PathBuf)>,
    to_load_recv: Receiver<(usize, PathBuf)>,
    loaded: Vec<Sender<(PathBuf, <<L as Loader>::Output as Source>::Output)>>,
    _phantom: PhantomData<L>,
}

impl<L: Loader> Builder<L> {
    /// Construct a new, empty `Builder`.
    #[allow(unused)]
    pub fn new() -> Self {
        let (to_load_send, to_load_recv) = channel();
        Self {
            to_load_send,
            to_load_recv,
            loaded: Vec::new(),
            _phantom: PhantomData::<L>,
        }
    }
    /// Create a new, empty `Manager<A>`.
    #[allow(unused)]
    pub fn create_manager<A: Asset<L>>(&mut self, data: A::DataManager) -> Manager<A, L> {
        let (s, r) = channel();
        let loader_id = self.loaded.len();
        self.loaded.push(s);
        Manager::new(loader_id, self.to_load_send.clone(), r, data)
    }

    /// Create the `Loader` associated with `Managers` built by this `Builder`.
    #[allow(unused)]
    pub fn finish_loader(self) -> L {
        L::new(self.to_load_recv, self.loaded)
    }
}
