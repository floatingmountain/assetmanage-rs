use crate::{
    loaders::{LoadStatus, Loader},
    sources::Source,
};
use std::{path::PathBuf, sync::Arc};

/// Any struct implementing the `Asset` trait can be Stored inside a corresponding `Manager`
pub trait Asset<L>
where
    L: Loader,
{
    type DataManager;
    type DataAsset;
    type Structure;
    fn construct(
        data_load: <L::Source as Source>::Output,
        data_ass: &Self::DataAsset,
        data_mgr: &Self::DataManager,
    ) -> Result<Self::Structure, std::io::Error>;
}

/// `AssetHandle` holds the Asset and its Metadata
#[derive(Clone)]
pub(crate) struct AssetHandle<A, L>
where
    A: Asset<L>,
    L: Loader,
{
    pub(crate) path: PathBuf,
    asset: Option<Arc<A::Structure>>,
    pub status: LoadStatus,
    pub data: A::DataAsset,
}

impl<A, L> AssetHandle<A, L>
where
    A: Asset<L>,
    L: Loader,
{
    pub(crate) fn new(path: PathBuf, data: A::DataAsset) -> Self {
        Self {
            path,
            asset: None,
            status: LoadStatus::NotLoaded,
            data,
        }
    }
    pub(crate) fn unload(&mut self) {
        self.asset = None;
        self.status = LoadStatus::NotLoaded;
    }
    pub(crate) fn set(&mut self, a: A::Structure) {
        self.asset = Some(Arc::new(a));
        self.status = LoadStatus::Loaded;
    }
    pub(crate) fn get(&self) -> Option<&Arc<A::Structure>> {
        self.asset.as_ref()
    }
}
