/// Block of the file system with inumber 0
/// Records meta-data about the entire file system
#[derive(Clone, Debug)]
pub struct FsSuperBlock {
    pub magic: u32,
    pub disk_size: u32,
}

impl From<&[u8]> for FsSuperBlock {
    fn from(super_block_bytes: &[u8]) -> Self {
        let magic = u32::from_le_bytes(crate::slice_to_four_bytes(&super_block_bytes[0..4]));
        let disk_size = u32::from_le_bytes(crate::slice_to_four_bytes(&super_block_bytes[4..8]));
        FsSuperBlock { magic, disk_size }
    }
}
