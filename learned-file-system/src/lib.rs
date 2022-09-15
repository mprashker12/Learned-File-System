use fuse::{Filesystem};
use std::os::raw::c_int;
use std::io::{Read, Write};

mod utils;

pub struct LearnedFileSystem {
    /// file descriptor for the disk
    disk_fd: usize,
    /// how many bytes in each block?
    fs_block_size: u32,
    fs_magic: u32,
    super_block: FsSuperBlock,
    bit_mask_block: BitMaskBlock,
}

/// Block of the file system with inumber 0
/// Records meta-data about the entire file system
pub struct FsSuperBlock {
    magic: u32,
    disk_size: u32,
    padding: Vec<u8>,
}

/// Block of the file system with inumber 1
/// Maintains which blocks are empty
pub struct BitMaskBlock {

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

    fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {

    }

    fn getattr(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyAttr) {
        
    }

    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, reply: fuse::ReplyDirectory) {
        
    }
}