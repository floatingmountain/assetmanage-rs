mod disk_source;
pub use disk_source::DiskSource;
use std::error::Error;

pub trait Source {
    type Input;
    type Output;
    fn load(item: Self::Input) -> Result<Self::Output, Box<dyn Error>>;
}
