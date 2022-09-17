use std::io::{Read, Write, Seek, SeekFrom};
use std::os::unix::fs::FileExt;
use std::fs::File;


pub trait BlockFile {
    fn block_size(&self) -> usize;
    fn num_blocks(&self) -> usize;

    fn block_read_in_place(&self, buf: &mut [u8], block_address: usize) -> std::io::Result<usize>;
    fn block_read(&self, block_address: usize) -> std::io::Result<Vec<u8>> {
        let mut output = vec![0; self.block_size()];
        self.block_read_in_place(&mut output, block_address)?;
        Ok(output)
    }

    

    fn block_write(&mut self, buf: &[u8], block_address: usize) -> std::io::Result<usize>;
}

pub struct BlockFileWrapper{
    block_size: usize,
    num_blocks: usize,
    file: File
}

impl BlockFileWrapper {
    pub fn new(block_size: usize, file: File) -> Self{
        let fsize = file.metadata().unwrap().len() as usize;
        let num_blocks = fsize.div_ceil(block_size);
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

    fn block_read_in_place(&self, mut buf: &mut [u8], block_address: usize) -> std::io::Result<usize> {
        let start = block_address*self.block_size;
        self.file.read_at(&mut buf, start as u64)
    }

    fn block_write(&mut self, buf: &[u8], block_address: usize) -> std::io::Result<usize> {
        let start = block_address*self.block_size;
        self.file.write_at(buf, start as u64)
    }
}
