use std::cell::RefCell;
use std::io::{Read, Write, Seek, SeekFrom, Error};
use std::os::unix::fs::FileExt;
use std::fs::File;
use crate::div_ceil;


pub trait BlockFile {
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> usize;

    fn block_read_in_place<T: AsMut<[u8]>>(&self, buf: T, block_address: usize) -> std::io::Result<usize>;
    fn block_read(&self, block_address: usize) -> std::io::Result<Vec<u8>> {
        let mut output = vec![0; self.block_size()];
        self.block_read_in_place(&mut output, block_address)?;
        Ok(output)
    }

    

    fn block_write<T : AsRef<[u8]>>(&mut self, buf: T, block_address: usize) -> std::io::Result<usize>;
}

pub struct BlockFileWrapper{
    block_size: usize,
    num_blocks: usize,
    file: File
}

impl BlockFileWrapper {
    pub fn new(block_size: usize, file: File) -> Self{
        let fsize = file.metadata().unwrap().len() as usize;
        let num_blocks = div_ceil(fsize, block_size);
        BlockFileWrapper {
            block_size, num_blocks, file
        }
    }
}

impl BlockFile for BlockFileWrapper {
    fn block_size(&self) -> usize {
        self.block_size
    }

    fn num_blocks(&self) -> usize {
        self.num_blocks
    }

    fn block_read_in_place<T: AsMut<[u8]>>(&self, mut buf: T, block_address: usize) -> std::io::Result<usize>{
        let start = block_address*self.block_size;
        self.file.read_at(buf.as_mut(), start as u64)
    }

    fn block_write<T : AsRef<[u8]>>(&mut self, buf: T, block_address: usize) -> std::io::Result<usize> {
        if buf.as_ref().len() != self.block_size{
            return Err(Error::from(std::io::ErrorKind::Other));
        }
        let start = block_address*self.block_size;
        self.file.write_at(buf.as_ref(), start as u64)
    }
}

pub struct LoggingBlockFileWrapper<T : BlockFile, W: Write>{
    inner: T,
    logger: RefCell<W>
}

impl <T: BlockFile, W: Write> LoggingBlockFileWrapper<T, W>{
    fn new(block_file: T, logger: W) -> Self{
        LoggingBlockFileWrapper{
            inner: block_file,
            logger: RefCell::new(logger)
        }
    }
}

impl <T: BlockFile, W: Write> BlockFile for LoggingBlockFileWrapper<T, W>{
    fn block_size(&self) -> usize {
        self.inner.block_size()
    }

    fn num_blocks(&self) -> usize {
        self.inner.num_blocks()
    }

    fn block_read_in_place<B: AsMut<[u8]>>(&self, buf: B, block_address: usize) -> std::io::Result<usize>{
        self.logger.borrow_mut().write(format!("R {}", block_address.to_string()).as_bytes())?;
        self.inner.block_read_in_place(buf, block_address)
    }

    fn block_read(&self, block_address: usize) -> std::io::Result<Vec<u8>> {
        self.logger.borrow_mut().write(format!("R {}", block_address.to_string()).as_bytes())?;
        self.inner.block_read(block_address)
    }

    fn block_write<B : AsRef<[u8]>>(&mut self, buf: B, block_address: usize) -> std::io::Result<usize> {
        self.logger.borrow_mut().write(format!("W {}", block_address.to_string()).as_bytes())?;
        self.inner.block_write(buf, block_address)
    }
}