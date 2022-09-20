use std::collections::BTreeSet;

const FS_BLOCK_SIZE: u32 = 4096; // TODO make this a runtime value, or maybe const generic?

/// Block of the file system with inumber 1
/// Maintains which blocks are empty
pub struct BitMaskBlock<> {
    bit_mask: [u8; FS_BLOCK_SIZE as usize],
    free_indices: BTreeSet<u32>,
    num_indices: usize
}

impl Default for BitMaskBlock {
    fn default() -> Self {
        let bit_mask = [255u8; FS_BLOCK_SIZE as usize];
        let mut free_indices = BTreeSet::new();

        BitMaskBlock { bit_mask, free_indices, num_indices: (FS_BLOCK_SIZE * 8) as usize }
    }
}

impl AsRef<[u8]> for BitMaskBlock{
    fn as_ref(&self) -> &[u8] {
        &self.bit_mask
    }
}

impl BitMaskBlock {
    pub fn new(num_blocks: usize, bit_mask_bytes: &[u8]) -> Self {
        let mut bit_mask = [0u8; FS_BLOCK_SIZE as usize];
        bit_mask.copy_from_slice(bit_mask_bytes);

        let mut free_indices = BTreeSet::<u32>::new();

        for index in 0..num_blocks {
            let bit_map_idx = index / 8;
            let bit_map_bit_idx = index % 8;
            if (bit_mask[bit_map_idx] >> bit_map_bit_idx) & 1 == 0{
                free_indices.insert(index as u32);
            }
        }
        BitMaskBlock {
            bit_mask, free_indices, num_indices: num_blocks
        }
    }

    pub fn set_bit(&mut self, index: u32) {
        if index >= self.num_indices as u32 {
            panic!("Trying to set bit {}, which is larger than {}", index, self.num_indices);
        }
        let byte_index = index/8;
        let byte_offset = index%8;
        self.bit_mask[byte_index as usize] |= 1 << byte_offset;
        self.free_indices.remove(&index);
    }

    pub fn is_free(&self, index: u32) -> bool {
        self.free_indices.contains(&index)
    }

    pub fn num_free_indices(&self) -> usize {
        self.free_indices.len()
    }

    pub fn clear_bit(&mut self, index: u32) {
        if index >= self.num_indices as u32 {
            panic!("Trying to clear bit {}, which is larger than {}", index, self.num_indices);
        }
        let byte_index = index/8;
        let byte_offset = index%8;
        self.bit_mask[byte_index as usize] &= 0 << byte_offset;
        self.free_indices.insert(index);
    }

    pub fn free_block_iter<'a>(&'a self) -> impl Iterator<Item=u32> + 'a {
        self.free_indices.iter().map(|a| *a)
    }
}