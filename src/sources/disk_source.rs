
use super::Source;
use std::{io::Read, path::PathBuf};

pub type DiskSource = ();

impl Source for DiskSource{
    type Input = PathBuf;
    type Output = Vec<u8>;
    fn load(path: Self::Input) -> Result<Self::Output, Box<dyn std::error::Error>> {
        let mut file = std::fs::File::open(&path)?;
        let mut contents = vec![];
        file.read_to_end(&mut contents)?;
        Ok(contents)
    }
}