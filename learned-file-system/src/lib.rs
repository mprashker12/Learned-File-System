use fuse::Filesystem;
use std::os::raw::c_int;

pub struct LearnedFileSystem {}

impl Filesystem for LearnedFileSystem {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        Ok(())
    }
}