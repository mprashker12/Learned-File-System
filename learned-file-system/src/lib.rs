#![feature(slice_as_chunks)]
#![feature(int_roundings)]

extern crate core;

pub mod utils;

use time::Timespec;
use fuse::{FileAttr, Filesystem};
use std::os::raw::c_int;
use std::collections::BTreeSet;
use fuse::FileType::{Directory, RegularFile};
use crate::utils::block_file::BlockFile;


const FS_BLOCK_SIZE: u32 = 4096;
const FS_MAGIC_NUM: u32 = 0x30303635;

/// Block of the file system with inumber 0
/// Records meta-data about the entire file system
pub struct FsSuperBlock {
    magic: u32,
    disk_size: u32,
}

pub struct FSINode {
    uid: u16,
    gid: u16,
    mode: u32,
    ctime: u32,
    mtime: u32,
    size: u32,
    pointers: Vec<u32> // [u32; ((FS_BLOCK_SIZE - 20)/4) as usize],
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
        let mut free_block_indices = BTreeSet::new(); // TODO populate this with data
        for index in 2..block_system.num_blocks() {
            free_block_indices.insert(index);
        }
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

fn slice_to_four_bytes(arr: &[u8]) -> [u8;4] {
    [arr[0], arr[1], arr[2], arr[3]]
}

fn slice_to_two_bytes(arr: &[u8]) -> [u8;2] {
    [arr[0], arr[1]]
}

impl From<&[u8]> for FsSuperBlock {
    fn from(super_block_bytes: &[u8]) -> Self {
        let magic = u32::from_le_bytes(slice_to_four_bytes(&super_block_bytes[0..4]));
        let disk_size = u32::from_le_bytes(slice_to_four_bytes(&super_block_bytes[4..8]));
        FsSuperBlock { magic, disk_size }
    }
}

impl From<&[u8]> for FSINode {
    fn from(inode_bytes: &[u8]) -> Self {
        let uid =  u16::from_le_bytes(slice_to_two_bytes(&inode_bytes[0..2]));
        let gid =  u16::from_le_bytes(slice_to_two_bytes(&inode_bytes[2..4]));
        let mode = u32::from_le_bytes(slice_to_four_bytes(&inode_bytes[4..8]));
        let ctime = u32::from_le_bytes(slice_to_four_bytes(&inode_bytes[8..12]));
        let mtime = u32::from_le_bytes(slice_to_four_bytes(&inode_bytes[12..16]));
        let size = u32::from_le_bytes(slice_to_four_bytes(&inode_bytes[16..20]));

        let pointers = inode_bytes[20..].as_chunks::<4>().0.iter()
            .map(|chunk| u32::from_le_bytes(*chunk))
            .collect();

        FSINode{
            uid, gid, mode, ctime, mtime, size, pointers,
        }
    }
}

fn time_to_timespec(time: u32) -> Timespec{
    Timespec{
        sec: time as i64,
        nsec: 0
    }
}

impl FSINode{
    fn to_fileattr(&self, node_num: u64) -> FileAttr {
        let dir_mask = 0o40000;

        let ftype = if self.mode & dir_mask != 0 { Directory } else { RegularFile };

        FileAttr{
            ino: node_num,
            uid: self.uid as u32,
            gid: self.gid as u32,
            mtime: time_to_timespec(self.mtime),
            ctime: time_to_timespec(self.ctime),
            crtime: time_to_timespec(self.ctime),
            atime: time_to_timespec(self.mtime),
            size: self.size as u64,
            blocks: (self.size as u64).div_ceil(FS_BLOCK_SIZE as u64),
            nlink: 1,
            rdev: 0,
            flags: 0,
            kind: ftype,
            perm: self.mode as u16
        }
    }
}



impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let super_block_data = self.block_system.block_read(0).map_err(|_| -1)?;
        let super_block = FsSuperBlock::from(super_block_data.as_slice());

        if super_block.magic != FS_MAGIC_NUM {return Err(-1)};
        // super_block.disk_size

        // self.super_block = super_block;
        // self.bit_mask_block = BitMaskBlock::default();
        Ok(())
    }
/*
    fn lookup(&mut self, _req: &fuse::Request, _parent: u64, _name: &std::ffi::OsStr, reply: fuse::ReplyEntry) {
        
    }
 */

    fn getattr(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyAttr) {
        let _ino = if _ino == 1 {2} else {_ino};
        let block_info = FSINode::from(self.block_system.block_read(_ino as usize).unwrap().as_slice());

        reply.attr(&time::get_time(), &block_info.to_fileattr(_ino))
    }

    fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {
        let super_block = FsSuperBlock::from(self.block_system.block_read(0).unwrap().as_slice());

        reply.statfs((super_block.disk_size - 2) as u64, self.free_block_indices.len() as u64,
                     self.free_block_indices.len() as u64, (super_block.disk_size - 2) as u64,
                     self.free_block_indices.len() as u64, FS_BLOCK_SIZE, 27,
                     FS_BLOCK_SIZE)
    }

    /*
    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, reply: fuse::ReplyDirectory) {
        
    }*/
}