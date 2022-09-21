pub mod utils;
mod structs;

use time::{Duration, get_time, Timespec};
use fuse::{FileAttr, Filesystem, FileType, FUSE_ROOT_ID, ReplyAttr, ReplyData, ReplyEmpty, ReplyEntry, ReplyWrite, Request};
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
use libc::{EEXIST, EIO, ENAMETOOLONG, ENOENT, ENOSPC};
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

fn translate_error(e : ErrorKind) -> c_int{
    match e {
        ErrorKind::OutOfMemory => ENOSPC,
        ErrorKind::AlreadyExists => EEXIST,
        _ => EIO
    }
}

fn translate_inode(ino: u64) -> u64{
    if ino == FUSE_ROOT_ID {2} else {ino}
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
        for block_index in block_indices.iter() {
            if self.block_allocation_bitmask.is_free(*block_index) {return Err(Error::from(Other));}
        }
        for block_index in block_indices {
            self.block_allocation_bitmask.clear_bit(block_index);
        }

        self.block_system.block_write(&self.block_allocation_bitmask, self.bit_mask_block_index)?;

        Ok(())
    }

    /// ASSUMPTION: The relevant block has already been allocated and initialized to 0
    fn write_file_chunk(&mut self, file: &FSINode, block_num_in_file: usize, offset: usize, data: &[u8]) -> std::io::Result<usize>{
        if offset + data.len() > FS_BLOCK_SIZE {
            panic!("Tried writing off end of file chunk");
        }

        let physical_block = file.pointers[block_num_in_file] as usize;

        if offset == 0 && data.len() == FS_BLOCK_SIZE{
            self.block_system.block_write(data, physical_block)
        } else{
            let mut pre_existing_chunk = self.block_system.block_read(physical_block)?;
            pre_existing_chunk[offset..(offset+data.len())].copy_from_slice(data);
            self.block_system.block_write(&pre_existing_chunk, physical_block)
        }

    }

    /// NOTE: Does not write back the inode itself
    fn write_file_data(&mut self, file: &mut FSINode, offset: usize, data: &[u8]) -> std::io::Result<usize>{
        let mut operations = vec![];

        let mut total_byte_writes_queued= 0;
        let mut total_allocations_needed = 0;
        let mut file_ptr = offset;

        while total_byte_writes_queued < data.len() {
            let logical_block_num = file_ptr / FS_BLOCK_SIZE;
            let block_offset = file_ptr % FS_BLOCK_SIZE;

            let write_length = if (FS_BLOCK_SIZE - block_offset) > (data.len() - total_byte_writes_queued) {
                data.len() - total_byte_writes_queued
            } else {
                FS_BLOCK_SIZE - block_offset
            };

            let allocation_num = if file.pointers[logical_block_num] == 0 {
                let anum = total_allocations_needed;
                total_allocations_needed += 1;
                Some(anum)
            } else{
                None
            };

            operations.push((&data[total_byte_writes_queued..(total_byte_writes_queued+write_length)], logical_block_num, block_offset, allocation_num));

            total_byte_writes_queued += write_length;
            file_ptr += write_length;
        }

        let allocations = self.allocate_blocks(total_allocations_needed)?;

        for (data_chunk, logical_blk_num, offset, allocation) in operations{
            if let Some(allocation_idx) = allocation {
                    file.pointers[logical_blk_num] = allocations[allocation_idx] as u32;
            }

            self.write_file_chunk(file, logical_blk_num, offset, data_chunk)?;
        }

        file.size = file.size.max(file_ptr as u32);

        Ok(total_byte_writes_queued)
    }

    fn read_file_chunk(&self, file: &FSINode, block_num_in_file: usize, offset: usize, mut dest: &mut [u8]){
        let disk_blknum = file.pointers[block_num_in_file] as usize;

        if disk_blknum == 0 { // Handle sparse/unallocated blocks
            dest.fill(0);
            return;
        }

        let len = dest.len();

        if offset + len > FS_BLOCK_SIZE {
            panic!("Tried reading off end of file chunk");
        }

        if offset == 0 && dest.len() == self.block_system.block_size() {
            self.block_system.block_read_in_place(&mut dest, disk_blknum).unwrap();
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
            for block in first_n_blocks.iter(){
                self.block_allocation_bitmask.set_bit(*block);
                self.block_system.block_write(&[0;FS_BLOCK_SIZE], *block as usize)?;
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

    fn find_dirent_in_list(&self, dirents_incl_gaps: &Vec<Result<DirectoryEntry, ()>>, name: &OsStr) -> Option<(usize, DirectoryEntry)>{
        dirents_incl_gaps.iter()
            .enumerate()
            .filter_map(|(idx, de)| de.clone().map(|de| (idx, de)).ok())
            .find(|(idx, de)| de.name == name)
    }

    fn first_free_dirent_idx(&self, dirent_incl_gaps: &Vec<Result<DirectoryEntry, ()>>) -> usize {
        dirent_incl_gaps.iter().enumerate().find(|(_, de)| de.is_err()).map(|(idx, _)| idx).unwrap_or(dirent_incl_gaps.len())
    }
}

//Main Implementations of the File System for LearnedFileSystem

impl <BF : BlockFile> Filesystem for LearnedFileSystem<BF> {
    
    fn init(&mut self, _req: &fuse::Request) -> Result<(), c_int> {
        let super_block = self.get_superblock().map_err(|e| translate_error(e.kind()))?;
        if super_block.magic != FS_MAGIC_NUM {return Err(-1)};

        let bitmask_block = self.block_system.block_read(self.bit_mask_block_index).map_err(|e| translate_error(e.kind()))?;
        self.block_allocation_bitmask = BitMaskBlock::new(super_block.disk_size as usize, &bitmask_block);

        Ok(())
    }

    fn lookup(&mut self, _req: &fuse::Request, _parent: u64, _name: &std::ffi::OsStr, reply: fuse::ReplyEntry) {
        let _ino = translate_inode(_parent);

        let block_info = self.get_inode(_ino).unwrap();
        if let Some((_, dirent)) = self.find_dirent_in_list(&self.get_dirents_incl_gaps(&block_info), _name){
            let element_block_info = FSINode::from(self.block_system.block_read(dirent.inode_ptr as usize).unwrap().as_slice());
            reply.entry(&in_one_sec(), &element_block_info.to_fileattr(dirent.inode_ptr as u64), 0);
        } else{
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &fuse::Request, orig_ino: u64, reply: fuse::ReplyAttr) {
        let _ino = translate_inode(orig_ino);
        let block_info = self.get_inode(_ino).unwrap();

        reply.attr(&in_one_sec(), &block_info.to_fileattr(orig_ino))
    }

    /*
    fn mknod(&mut self, _req: &Request, _orig_parent: u64, _name: &OsStr, _mode: u32, _rdev: u32, reply: ReplyEntry) {
        let _parent = translate_inode(_orig_parent);
        todo!()
    }
     */

    fn mkdir(&mut self, _req: &Request, _orig_parent: u64, _name: &OsStr, _mode: u32, reply: ReplyEntry) {
        let _parent = translate_inode(_orig_parent);
        if _name.as_bytes().len() > 27 {
            reply.error(ENAMETOOLONG);
            return;
        }

        let mut parent_inode = self.get_inode(_parent).unwrap();
        let parent_dirents = self.get_dirents_incl_gaps(&parent_inode);
        let first_free_parent_dirent_idx = self.first_free_dirent_idx(&parent_dirents);

        if let Some(_) = self.find_dirent_in_list(&parent_dirents, _name) {
            reply.error(EEXIST);
            return;
        }

        match self.allocate_blocks(1){
            Ok(newdir_blocks) => {

                let newdir_inode_blknum = newdir_blocks[0];
                let now_sec = get_time().sec;

                let new_inode = FSINode {
                    pointers: [0u32; NUM_POINTERS],
                    size: 0,
                    uid: _req.uid() as u16,
                    gid: _req.gid() as u16,
                    mode: _mode,
                    ctime: now_sec as u32,
                    mtime: now_sec as u32
                };

                let ino_data: Vec<u8> = new_inode.clone().into();
                if let Err(e) = self.block_system.block_write(&ino_data, newdir_inode_blknum as usize){
                    reply.error(translate_error(e.kind()));
                    return;
                }

                let dirent = DirectoryEntry{
                    inode_ptr: newdir_inode_blknum as u32,
                    name: OsString::from(_name)
                };

                let dirent_data: Vec<u8> = dirent.into();
                if let Err(e) = self.write_file_data(&mut parent_inode, first_free_parent_dirent_idx*32, &dirent_data){
                    reply.error(translate_error(e.kind()));
                    return;
                }

                let parent_inode_data : Vec<u8> = parent_inode.into();
                if let Err(e) = self.block_system.block_write(&parent_inode_data, _parent as usize){
                    reply.error(translate_error(e.kind()));
                    return;
                }

                reply.entry(&in_one_sec(), &new_inode.to_fileattr(newdir_inode_blknum as u64), 0)
            }
            Err(e) => {
                reply.error(translate_error(e.kind()))
            }
        }
    }

    fn read(&mut self, _req: &Request, _orig_ino: u64, _fh: u64, _offset: i64, _size: u32, reply: ReplyData) {
        let _ino = translate_inode(_orig_ino);
        let block_info = self.get_inode(_ino).unwrap();
        let data = self.read_file_bytes(&block_info, _offset as usize, _size as usize);
        reply.data(&data)
    }

    fn rename(&mut self, _req: &Request, _parent: u64, _name: &OsStr, _newparent: u64, _newname: &OsStr, reply: ReplyEmpty) {
        let parent_ino = translate_inode(_parent);
        let new_parent_ino = translate_inode(_newparent);

        if _newname.as_bytes().len() > 27 {
            reply.error(ENAMETOOLONG);
            return;
        }

        let mut old_parent_info = self.get_inode(parent_ino).unwrap();
        let old_parent_dirents = self.get_dirents_incl_gaps(&old_parent_info);

        match self.find_dirent_in_list(&old_parent_dirents, _name){
            Some((old_de_idx, mut dirent)) => {
                dirent.name = OsString::from(_newname);

                if new_parent_ino == parent_ino {
                    if _newname == _name {
                        reply.ok();
                        return;
                    }


                    let dirent_data: Vec<u8> = dirent.into();
                    if let Err(e) = self.write_file_data(&mut old_parent_info, old_de_idx*32, &dirent_data){
                        reply.error(translate_error(e.kind()));
                        return;
                    }

                    let parent_inode_data : Vec<u8> = old_parent_info.into();
                    if let Err(e) = self.block_system.block_write(&parent_inode_data, parent_ino as usize){
                        reply.error(translate_error(e.kind()));
                        return;
                    }

                    reply.ok();
                    return;
                } else {
                    let mut new_parent_info = self.get_inode(new_parent_ino).unwrap();
                    let new_parent_dirents = self.get_dirents_incl_gaps(&new_parent_info);

                    if let Some(_) = self.find_dirent_in_list(&new_parent_dirents, _name) {
                        reply.error(EEXIST);
                        return;
                    }


                    if let Err(e) = self.write_file_data(&mut old_parent_info, old_de_idx*32, &[0u8; 32]){
                        reply.error(translate_error(e.kind()));
                        return;
                    }

                    let parent_inode_data : Vec<u8> = old_parent_info.into();
                    if let Err(e) = self.block_system.block_write(&parent_inode_data, parent_ino as usize){
                        reply.error(translate_error(e.kind()));
                        return;
                    }

                    let first_free_new_parent_dirent_idx = self.first_free_dirent_idx(&new_parent_dirents);
                    let dirent_data: Vec<u8> = dirent.into();
                    if let Err(e) = self.write_file_data(&mut new_parent_info, first_free_new_parent_dirent_idx*32, &dirent_data){
                        reply.error(translate_error(e.kind()));
                        return;
                    }

                    let new_parent_inode_data : Vec<u8> = new_parent_info.into();
                    if let Err(e) = self.block_system.block_write(&new_parent_inode_data, new_parent_ino as usize){
                        reply.error(translate_error(e.kind()));
                        return;
                    }
                    reply.ok()
                }
            }
            None => {
                reply.error(ENOENT);
            }
        }
    }

    fn write(&mut self, _req: &Request, _orig_ino: u64, _fh: u64, _offset: i64, _data: &[u8], _flags: u32, reply: ReplyWrite) {
        let _ino = translate_inode(_orig_ino);

        let mut block_info = self.get_inode(_ino).unwrap();

        match self.write_file_data(&mut block_info, _offset as usize, _data){
            Err(e) => {reply.error(translate_error(e.kind()))},
            Ok(bytes_written) => {
                let parent_inode_data : Vec<u8> = block_info.into();
                if let Err(e) = self.block_system.block_write(&parent_inode_data, _ino as usize){
                    reply.error(translate_error(e.kind()));
                    return;
                }

                reply.written(bytes_written as u32);
            }
        }
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

