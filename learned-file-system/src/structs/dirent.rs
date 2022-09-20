use std::ffi::{CString, OsString};
use std::num::NonZeroU8;
use std::ops::Deref;
use crate::FS_BLOCK_SIZE;

pub struct DirectoryBlock {
    pub directory_entries: [DirectoryEntry; (FS_BLOCK_SIZE/4)],
}

pub struct DirectoryEntry {
    pub inode_ptr: u32,
    pub name: OsString,
}

impl TryFrom<&[u8]> for DirectoryEntry{

    type Error = ();

    fn try_from(dirent: &[u8]) -> Result<Self, Self::Error> {
        let valid = dirent[0] & 0x01 != 0;
        if valid {
            let inode_ptr = u32::from_le_bytes(crate::slice_to_four_bytes(&dirent[0..4])) >> 1; // Compensate for the valid bit
            let name_nonzero: Vec<NonZeroU8> = dirent[4..32].iter().map_while(|ch| NonZeroU8::new(*ch)).collect();
            let cname = CString::from(name_nonzero);
            let name = OsString::from(cname.to_string_lossy().deref());
            Ok(DirectoryEntry{
                inode_ptr, name
            })
        } else {
            Err(())
        }
    }
}
