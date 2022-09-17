#![feature(slice_as_chunks)]
#![feature(int_roundings)]


pub mod utils;

use time::{Duration, Timespec};
use fuse::{FileAttr, Filesystem, FileType, FUSE_ROOT_ID};
use utils::bitmask::BitMaskBlock;
use std::os::raw::c_int;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString, OsString};
use std::num::NonZeroU8;
use std::ops::{Add, Deref};
use fuse::FileType::{Directory, RegularFile};
use crate::utils::block_file::BlockFile;
use libc::ENOENT;


const FS_BLOCK_SIZE: usize = 4096;
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
    directory_entries: [DirectoryEntry; (FS_BLOCK_SIZE/4)],
}

pub struct DirectoryEntry {
    valid: bool,
    inode_ptr: u32,
    name: CString,
}


pub struct LearnedFileSystem <BF : BlockFile> {
    block_system: BF,
    free_block_indices: BTreeSet<usize>,
    super_block_index: usize,
    bit_mask_block_index: usize,
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

impl <BF: BlockFile>  LearnedFileSystem<BF> {
    pub fn new(block_system: BF) -> Self {
        let mut free_block_indices = BTreeSet::new();
        for index in 2..block_system.num_blocks() {
            free_block_indices.insert(index);
        }
        LearnedFileSystem {
            block_system,
            free_block_indices,
            super_block_index: 0,
            bit_mask_block_index: 1,
        }
    }

     /// Read Bitmask block from disk, clear all bits given, and write bitmask
    /// block back to disk
    pub fn free_blocks(&mut self, block_indices: Vec<usize>) -> bool {
        for block_index in block_indices.iter() {
            if self.free_block_indices.contains(block_index) {return false;}
        }
        let mut bit_mask_block = BitMaskBlock::from(
            self.block_system.block_read(self.bit_mask_block_index).unwrap().as_slice()
        );
        for block_index in block_indices.iter() {
            bit_mask_block.clear_bit(*block_index as u32);
        }
        let res = self.block_system.block_write(&bit_mask_block.bit_mask.clone(), self.bit_mask_block_index);
        if res.is_err() || res.unwrap() != FS_BLOCK_SIZE {return false;}
        for block_index in block_indices.iter() {
            self.free_block_indices.insert(*block_index);
        }
        true
    }

    pub fn first_free_block(&self) -> u32 {
        let mut free_block_iter = self.free_block_indices.iter();
        if let Some(first_free_index) = free_block_iter.next() {
            return *first_free_index as u32;
        }
        panic!("Trying to find a free block when all blocks are full");
    }

    fn read_file_chunk(&self, file: &FSINode, block_num_in_file: usize, offset: usize, dest: &mut [u8]){
        let disk_blknum = file.pointers[block_num_in_file] as usize;

        if offset == 0 && dest.len() == self.block_system.block_size() {
            self.block_system.block_read_in_place(dest, disk_blknum).unwrap();
        }

        if offset + dest.len() > FS_BLOCK_SIZE {
            panic!("Tried reading off end of file chunk");
        }
        let blk = self.block_system.block_read(disk_blknum).unwrap();
        dest.copy_from_slice(&blk[offset..(offset+dest.len())])
    }

    fn read_file_bytes_in_place(&self, file: &FSINode, offset: usize, dest: &mut [u8]) -> usize {
        let len = if (dest.len() + offset) > file.size as usize {
            file.size as usize - offset
        } else {
            dest.len()
        };

        let mut file_ptr = offset;
        let mut total_num_read = 0;

        while total_num_read < len {
            let block_num = file_ptr / FS_BLOCK_SIZE;
            let block_offset = file_ptr % FS_BLOCK_SIZE;

            let read_length = if (FS_BLOCK_SIZE - block_offset) > (len - total_num_read) {
                len - total_num_read
            } else {
                FS_BLOCK_SIZE - block_offset
            };

            self.read_file_chunk(file, block_num, block_offset, &mut dest[total_num_read..(total_num_read + read_length)]);
            total_num_read += read_length;
            file_ptr += read_length;
        }

        total_num_read
    }

    fn read_file_bytes(&self, file: &FSINode, offset: usize, len: usize) -> Vec<u8> {
        let mut dest = vec![0u8; len];
        let num_read = self.read_file_bytes_in_place(file, offset, &mut dest);
        dest.truncate(num_read);
        dest
    }

}

//Main Implementations of the File System for LearnedFileSystem

impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let super_block_data = self.block_system.block_read(self.super_block_index).map_err(|_| -1)?;
        let super_block = FsSuperBlock::from(super_block_data.as_slice());
        if super_block.magic != FS_MAGIC_NUM {return Err(-1)};
        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, _parent: u64, _name: &std::ffi::OsStr, reply: fuse::ReplyEntry) {
        let _ino = if _parent == FUSE_ROOT_ID {2} else {_parent};

        let block_info = FSINode::from(self.block_system.block_read(_ino as usize).unwrap().as_slice());
        let dir_contents = self.read_file_bytes(&block_info, 0, block_info.size as usize);
        for dirent in dir_contents.chunks_exact(32).map(DirectoryEntry::from).filter(|dir| dir.valid) {
            let name = OsString::from(dirent.name.to_string_lossy().deref());
            if name == _name {
                let element_block_info = FSINode::from(self.block_system.block_read(dirent.inode_ptr as usize).unwrap().as_slice());
                reply.entry(&in_one_sec(), &element_block_info.to_fileattr(dirent.inode_ptr as u64), 0);
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &fuse::Request, orig_ino: u64, reply: fuse::ReplyAttr) {
        let _ino = if orig_ino == FUSE_ROOT_ID {2} else {orig_ino};
        let block_info = FSINode::from(self.block_system.block_read(_ino as usize).unwrap().as_slice());

        reply.attr(&in_one_sec(), &block_info.to_fileattr(orig_ino))
    }

    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, mut reply: fuse::ReplyDirectory) {
        let _ino = if _ino == 1 {2} else {_ino};

        let block_info = FSINode::from(self.block_system.block_read(_ino as usize).unwrap().as_slice());
        let dir_contents = self.read_file_bytes(&block_info, 0, block_info.size as usize);
        for dirent in dir_contents.chunks_exact(32).map(DirectoryEntry::from).filter(|dir| dir.valid) {
            for (off, dirent) in dir_contents.chunks_exact(32).map(|raw_fsdir| DirectoryEntry::from(raw_fsdir)).filter(|dir| dir.valid).enumerate().skip(_offset as usize) {
                let name = OsString::from(dirent.name.to_string_lossy().deref());
                reply.add(dirent.inode_ptr as u64, (off + 1) as i64, Directory, &name);
            }
        }
        reply.ok()
    }

    fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {
        let super_block = FsSuperBlock::from(self.block_system.block_read(0).unwrap().as_slice());

        reply.statfs((super_block.disk_size - 2) as u64, self.free_block_indices.len() as u64,
                     self.free_block_indices.len() as u64, (super_block.disk_size - 2) as u64,
                     self.free_block_indices.len() as u64, FS_BLOCK_SIZE as u32, 27,
                     FS_BLOCK_SIZE as u32);
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

impl From<&[u8]> for BitMaskBlock {
    fn from(bit_mask_bytes: &[u8]) -> Self {
        let mut bit_mask = [0u8; FS_BLOCK_SIZE];
        for (index, byte) in bit_mask_bytes.iter().enumerate() {
            bit_mask[index] = *byte;
        }
        BitMaskBlock {
            bit_mask
        }
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

impl From<&[u8]> for DirectoryEntry{
    fn from(dirent: &[u8]) -> Self {
        let valid = dirent[0] & 0x80 != 0;
        let inode_ptr = u32::from_le_bytes(slice_to_four_bytes(&dirent[0..4])) & 0x7FFFFFFF;
        let name_nonzero: Vec<NonZeroU8> = dirent[4..32].iter().map_while(|ch| NonZeroU8::new(*ch)).collect();
        let name = CString::from(name_nonzero);
        DirectoryEntry{
            valid, inode_ptr, name
        }
    }
}

fn time_to_timespec(time: u32) -> Timespec{
    Timespec{
        sec: time as i64,
        nsec: 0
    }
}


fn in_one_sec() -> Timespec{
    time::get_time().add(Duration::seconds(1))
}

