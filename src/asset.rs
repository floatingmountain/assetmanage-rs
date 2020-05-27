use crate::loader::LoadStatus;
use std::{path::PathBuf, sync::Arc};

/// Any struct implementing the `Asset` trait can be Stored inside a corresponding `Manager`
pub trait Asset
{
    type Output;
    fn decode(bytes: &[u8]) -> Result<Self::Output, std::io::Error>;
}

/// `AssetHandle` holds the Asset and its Metadata
#[derive(Clone)]
pub(crate) struct AssetHandle<A>
where
    A: Asset,
{
    pub(crate) path: PathBuf,
    asset: Option<Arc<A::Output>>,
    pub status: LoadStatus,
}

impl<A: Asset> AssetHandle<A> {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self {
            path,
            asset: None,
            status: LoadStatus::NotLoaded,
        }
    }
    pub(crate) fn unload(&mut self) {
        self.asset = None;
        self.status = LoadStatus::NotLoaded;
    }
    pub(crate) fn set(&mut self, a: A::Output) {
        self.asset = Some(Arc::new(a));
        self.status = LoadStatus::Loaded;
    }
    pub(crate) fn get(&self) -> Option<&Arc<A::Output>> {
        self.asset.as_ref()
    }
}
