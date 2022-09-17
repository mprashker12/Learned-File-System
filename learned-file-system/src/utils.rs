pub mod block_file;
pub mod bitmask;

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
