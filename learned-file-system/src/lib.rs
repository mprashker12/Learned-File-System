
pub mod utils;

use fuse::{Filesystem};
use std::os::raw::c_int;
use std::collections::BTreeSet;
use crate::utils::block_file::BlockFile;


const FS_BLOCK_SIZE: u32 = 4096;
const FS_MAGIC_NUM: u32 = 0; // TODO

/// Block of the file system with inumber 0
/// Records meta-data about the entire file system
pub struct FsSuperBlock {
    magic: u32,
    /// How many blocks is the entire disk?
    disk_size: u32,
    /// Dummy bytes to make this struct the size of a block
    padding: [u8; (FS_BLOCK_SIZE as usize - 8)],
}

pub struct FSINode {
    uid: u16,
    gid: u16,
    mode: u32,
    ctime: u32,
    mtime: u32,
    size: u32,
    pointers: [u32; ((FS_BLOCK_SIZE - 20)/4) as usize],
}

pub struct DirectoryBlock {
    directory_entries: [DirectoryEntry; (FS_BLOCK_SIZE/4) as usize],
}

pub struct DirectoryEntry {
    valid: bool,
    inode_ptr: u32,
    name: [char; 28],
}


pub struct LearnedFileSystem <BF : BlockFile> {
    block_system: BF,
    free_block_indices: BTreeSet<usize>,
}

impl <BF: BlockFile>  LearnedFileSystem<BF> {
    pub fn new(block_system: BF) -> Self {
        let free_block_indices = BTreeSet::new(); // TODO populate this with data
        LearnedFileSystem {
            block_system,
            free_block_indices
        }
    }

    pub fn first_free_block(&self) -> u32 {
        let mut free_block_iter = self.free_block_indices.iter();
        if let Some(first_free_index) = free_block_iter.next() {
            return *first_free_index as u32;
        }
        panic!("Trying to find a free block when all blocks are full");
    }
}



impl From<&[u8]> for FsSuperBlock {
    fn from(super_block_bytes: &[u8]) -> Self {
        let magic = (1 << 24)*(super_block_bytes[0] as u32)
                        + (1 << 16)*(super_block_bytes[1] as u32)
                        + (1 << 8)*(super_block_bytes[2] as u32)
                        + (super_block_bytes[3] as u32);

    
        let disk_size = (1 << 24)*(super_block_bytes[4] as u32)
                        + (1 << 16)*(super_block_bytes[5] as u32)
                        + (1 << 8)*(super_block_bytes[6] as u32)
                        + (super_block_bytes[7] as u32);

        let padding = [0u8; (FS_BLOCK_SIZE as usize - 8)];

        FsSuperBlock { magic, disk_size, padding }
    }
}

// impl From<&[u8]> for FSINode {
//     fn from(_: &[u8]) -> Self {
//         //TODO
//     }
// }



impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let super_block_data = self.block_system.block_read(0).map_err(|e| -1)?;
        let super_block = FsSuperBlock::from(super_block_data.as_slice());

        if super_block.magic != FS_MAGIC_NUM {return Err(-1)};
        // super_block.disk_size

        // self.super_block = super_block;
        // self.bit_mask_block = BitMaskBlock::default();
        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, _parent: u64, _name: &std::ffi::OsStr, reply: fuse::ReplyEntry) {
        
    }


    /*fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {
        
    }

    fn getattr(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyAttr) {
        
    }

    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, reply: fuse::ReplyDirectory) {
        
    }*/
}