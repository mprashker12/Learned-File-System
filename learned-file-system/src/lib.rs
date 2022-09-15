
pub mod utils;

use fuse::{Filesystem};
use std::os::raw::c_int;
use std::collections::BTreeSet;
use crate::utils::block_file::BlockFile;


const FS_BLOCK_SIZE: u32 = 4096;


pub struct LearnedFileSystem <BF : BlockFile> {
    /// file descriptor for the disk
    block_system: BF,
    free_block_indices: BTreeSet<usize>,
    // fs_magic: u32,
    // super_block: FsSuperBlock,
    // bit_mask_block: BitMaskBlock,
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
    pointers: Vec<usize>,
}

/// Block of the file system with inumber 1
/// Maintains which blocks are empty
pub struct BitMaskBlock<> {
    bit_mask: [u8; FS_BLOCK_SIZE as usize],
}

impl Default for BitMaskBlock {
    fn default() -> Self {
        let bit_mask = [0u8; FS_BLOCK_SIZE as usize];
        BitMaskBlock { bit_mask }
    }
}

impl BitMaskBlock {

    pub fn set_bit(&mut self, index: u32) {
        if index >= 8*FS_BLOCK_SIZE {
            panic!("Trying to set bit {}, which is larger than {}", index, 8*FS_BLOCK_SIZE);
        }
        let byte_index = index/8;
        let byte_offset = index%8;
        self.bit_mask[byte_index as usize] &= 1 << byte_offset;
    }

    pub fn clear_bit(&mut self, index: u32) {
        if index >= 8*FS_BLOCK_SIZE {
            panic!("Trying to clear bit {}, which is larger than {}", index, 8*FS_BLOCK_SIZE);
        }
        let byte_index = index/8;
        self.bit_mask[byte_index as usize] = 0;
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



impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let mut buf = [0 as u8; FS_BLOCK_SIZE as usize];
        let res = utils::block_read(&mut buf, FS_BLOCK_SIZE, 0, DISK_FD);
        if res.is_err() {return Err(-1);}

        let super_block = FsSuperBlock::from(buf.as_slice());
        if super_block.magic != self.fs_magic {return Err(-1);}

        self.super_block = super_block;
        self.bit_mask_block = BitMaskBlock::default();
        Ok(())
    }

    /*fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {

    }

    fn getattr(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyAttr) {
        
    }

    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, reply: fuse::ReplyDirectory) {
        
    }*/
}