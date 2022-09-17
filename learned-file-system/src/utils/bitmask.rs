const FS_BLOCK_SIZE: u32 = 4096; // TODO make this a runtime value, or maybe const generic?

/// Block of the file system with inumber 1
/// Maintains which blocks are empty
pub struct BitMaskBlock<> {
    pub bit_mask: [u8; FS_BLOCK_SIZE as usize],
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
        let byte_offset = index%8;
        self.bit_mask[byte_index as usize] &= 0 << byte_offset;
    }
}