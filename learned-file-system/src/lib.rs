use fuse::{Filesystem, mount};
use std::os::raw::c_int;

pub struct LearnedFileSystem {
    FS_BLOCK_SIZE: u32,
    FS_MAGIC: u32,
    SuperBlock: FsSuperBlock,
}

pub struct FsSuperBlock {
    magic: u32,
    disk_size: u32,
    padding: Vec<u8>,
}

pub struct FSINode {
    uid: u16,
    gid: u16,
    mode: u32,
    ctime: u32,
    mtime: u32,
    size: u32,
    pointers: Vec<usize>,
}



impl Filesystem for LearnedFileSystem {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        Ok(())
    }
}