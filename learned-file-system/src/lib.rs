pub mod utils;
mod structs;

use time::{Duration, get_time, Timespec};
use fuse::{FileAttr, Filesystem, FileType, FUSE_ROOT_ID, ReplyData, ReplyEntry, ReplyWrite, Request};
use utils::bitmask::BitMaskBlock;
use std::os::raw::c_int;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString, OsStr, OsString};
use std::io::{Error, ErrorKind};
use std::io::ErrorKind::{Other, OutOfMemory};
use std::num::NonZeroU8;
use std::ops::{Add, Deref};
use std::os::unix::ffi::OsStrExt;
use fuse::FileType::{Directory, RegularFile};
use crate::utils::block_file::BlockFile;
use libc::{EEXIST, ENAMETOOLONG, ENOENT, ENOSPC};
use structs::dirent::DirectoryEntry;
use structs::fsinode::FSINode;
use structs::superblock::FsSuperBlock;
use crate::structs::fsinode::NUM_POINTERS;
use crate::utils::div_ceil;


const FS_BLOCK_SIZE: usize = 4096;
const FS_MAGIC_NUM: u32 = 0x30303635;


pub struct LearnedFileSystem <BF : BlockFile> {
    block_system: BF,
    block_allocation_bitmask: BitMaskBlock,
    super_block_index: usize,
    bit_mask_block_index: usize,
}

impl <BF: BlockFile>  LearnedFileSystem<BF> {
    pub fn new(block_system: BF) -> Self {
        let block_allocation_bitmask = BitMaskBlock::default();

        LearnedFileSystem {
            block_system,
            block_allocation_bitmask,
            super_block_index: 0,
            bit_mask_block_index: 1,
        }
    }

     /// Read Bitmask block from disk, clear all bits given, and write bitmask
     /// block back to disk
    pub fn free_blocks(&mut self, block_indices: Vec<u32>) -> std::io::Result<()> {
        for block_index in block_indices {
            if self.block_allocation_bitmask.is_free(block_index) {return Err(Error::from(Other));}
        }
        for block_index in block_indices {
            self.block_allocation_bitmask.clear_bit(block_index);
        }

        self.block_system.block_write(&self.block_allocation_bitmask, self.bit_mask_block_index)?;

        Ok(())
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

    fn get_superblock(&self) -> std::io::Result<FsSuperBlock>{
        Ok(FsSuperBlock::from(self.block_system.block_read(0)?.as_slice()))
    }

    fn get_inode(&self, inode: u64) -> std::io::Result<FSINode>{
        Ok(FSINode::from(self.block_system.block_read(inode as usize)?.as_slice()))
    }

    fn allocate_blocks(&mut self, num_blocks: usize) -> std::io::Result<Vec<u32>>{
        let first_n_blocks : Vec<u32> = self.block_allocation_bitmask.free_block_iter().take(num_blocks).collect();
        if first_n_blocks.len() == num_blocks {
            for block in first_n_blocks{
                self.block_allocation_bitmask.set_bit(block)
            }
            self.block_system.block_write(&self.block_allocation_bitmask, self.bit_mask_block_index)?;
            Ok(first_n_blocks)
        } else{
            Err(Error::from(OutOfMemory))
        }
    }

    fn get_dirents_incl_gaps(&self, block_info: &FSINode) -> Vec<Result<DirectoryEntry, ()>>{
        let dir_contents = self.read_file_bytes(&block_info, 0, block_info.size as usize);
        dir_contents.chunks_exact(32).map(DirectoryEntry::try_from).collect()
    }

    fn get_valid_dirents(&self, block_info: &FSINode) -> Vec<DirectoryEntry>{
        self.get_dirents_incl_gaps(block_info).into_iter().filter_map(Result::ok).collect()
    }
}

//Main Implementations of the File System for LearnedFileSystem

impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let super_block = self.get_superblock().map_err(|_| -1)?;
        if super_block.magic != FS_MAGIC_NUM {return Err(-1)};

        let bit_mask_block = self.block_system.block_read(self.bit_mask_block_index).map_err(|_| -1)?;


        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, _parent: u64, _name: &std::ffi::OsStr, reply: fuse::ReplyEntry) {
        let _ino = if _parent == FUSE_ROOT_ID {2} else {_parent};

        let block_info = self.get_inode(_ino).unwrap();
        for dirent in self.get_valid_dirents(&block_info) {
            if dirent.name == _name {
                let element_block_info = FSINode::from(self.block_system.block_read(dirent.inode_ptr as usize).unwrap().as_slice());
                reply.entry(&in_one_sec(), &element_block_info.to_fileattr(dirent.inode_ptr as u64), 0);
                return;
            }
        }
        reply.error(ENOENT);
    }

    fn getattr(&mut self, _req: &fuse::Request, orig_ino: u64, reply: fuse::ReplyAttr) {
        let _ino = if orig_ino == FUSE_ROOT_ID {2} else {orig_ino};
        let block_info = self.get_inode(_ino).unwrap();

        reply.attr(&in_one_sec(), &block_info.to_fileattr(orig_ino))
    }

    fn mknod(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _mode: u32, _rdev: u32, reply: ReplyEntry) {
        todo!()
    }

    fn mkdir(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _mode: u32, reply: ReplyEntry) {
        if _name.as_bytes().len() > 27 {
            reply.error(ENAMETOOLONG);
            return;
        }

        let parent_inode = self.get_inode(_parent).unwrap();
        let parent_dirents = self.get_dirents_incl_gaps(&parent_inode);
        let first_free_parent_dirent_idx = parent_dirents.iter().enumerate().find(|(_, de)| de.is_err()).map(|(idx, _)| idx).unwrap_or(parent_dirents.len());

        for existing_dirent in parent_dirents.into_iter().filter_map(Result::ok){
            if existing_dirent.name == _name {
                reply.error(EEXIST);
                return;
            }
        }

        match self.allocate_blocks(1){
            Ok(newdir_blocks) => {

                let newdir_inode_blknum = newdir_blocks[0];
                let now_sec = get_time().sec;

                let new_inode = FSINode {
                    pointers: [0u32; NUM_POINTERS],
                    size: FS_BLOCK_SIZE as u32,
                    uid: _req.uid() as u16,
                    gid: _req.gid() as u16,
                    mode: _mode,
                    ctime: now_sec as u32,
                    mtime: now_sec as u32
                };

                let ino_data: Vec<u8> = new_inode.into();
                self.block_system.block_write(&ino_data, newdir_inode_blknum as usize).unwrap();

                let dirent = DirectoryEntry{
                    inode_ptr: newdir_inode_blknum as u32,
                    name: OsString::from(_name)
                };

                let dirent_data: Vec<u8> = dirent.into();
                self.write_to_file(&parent_inode, &dirent_data, first_free_parent_dirent_idx*32);
                reply.entry(&in_one_sec(), &new_inode.to_fileattr(newdir_inode_blknum as u64), 0)
            }
            Err(e) => {
                reply.error(ENOSPC)
            }
        }
    }

    fn read(&mut self, _req: &Request, _ino: u64, _fh: u64, _offset: i64, _size: u32, reply: ReplyData) {
        let block_info = self.get_inode(_ino).unwrap();
        let data = self.read_file_bytes(&block_info, _offset as usize, _size as usize);
        reply.data(&data)
    }

    fn write(&mut self, _req: &Request, _ino: u64, _fh: u64, _offset: i64, _data: &[u8], _flags: u32, reply: ReplyWrite) {
        todo!()
    }

    fn readdir(&mut self, _req: &fuse::Request, _ino: u64, _fh: u64, _offset: i64, mut reply: fuse::ReplyDirectory) {
        let _ino = if _ino == FUSE_ROOT_ID {2} else {_ino};
        let block_info = self.get_inode(_ino).unwrap();
        for (off, dirent) in self.get_valid_dirents(&block_info).into_iter().enumerate().skip(_offset as usize) {
            reply.add(dirent.inode_ptr as u64, (off + 1) as i64, Directory, &dirent.name);
        }
        reply.ok()
    }

    fn statfs(&mut self, _req: &fuse::Request, _ino: u64, reply: fuse::ReplyStatfs) {
        let super_block = self.get_superblock().unwrap();

        reply.statfs((super_block.disk_size - 2) as u64, self.block_allocation_bitmask.num_free_indices() as u64,
                     self.block_allocation_bitmask.num_free_indices() as u64, (super_block.disk_size - 2) as u64,
                     self.block_allocation_bitmask.num_free_indices() as u64, FS_BLOCK_SIZE as u32, 27,
                     FS_BLOCK_SIZE as u32);
    }

}

fn slice_to_four_bytes(arr: &[u8]) -> [u8;4] {
    [arr[0], arr[1], arr[2], arr[3]]
}

fn slice_to_two_bytes(arr: &[u8]) -> [u8;2] {
    [arr[0], arr[1]]
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

