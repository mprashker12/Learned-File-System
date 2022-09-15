use std::io::{Read, Write, Seek, SeekFrom};
use std::os::unix::io::FromRawFd;
use std::fs::File;


pub fn block_read(
    buf: &mut [u8], 
    block_size: u32, 
    logical_block_address: u32,
    disk_fd: i32,
) -> std::io::Result<usize> {
    let mut disk = unsafe { File::from_raw_fd(disk_fd) };
    let start = logical_block_address*block_size;
    disk.seek(SeekFrom::Start(start.try_into().unwrap()))?;
    let bytes_read = disk.read(buf)?;
    Ok(bytes_read)
}

pub fn block_write(
    buf: &[u8],
    block_size: u32,
    logical_block_address: u32,
    disk_fd: i32,
) -> std::io::Result<usize> {
    let mut disk = unsafe { File::from_raw_fd(disk_fd) };
    let start = logical_block_address*block_size;
    disk.seek(SeekFrom::Start(start.try_into().unwrap()))?;
    let bytes_written = disk.write(buf)?;
    Ok(bytes_written)
}