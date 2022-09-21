use fuse::FileAttr;
use fuse::FileType::{Directory, RegularFile};
use crate::{div_ceil, FS_BLOCK_SIZE};

pub const NUM_POINTERS: usize = ((FS_BLOCK_SIZE - 20)/4) as usize;

#[derive(Clone)]
pub struct FSINode {
    pub uid: u16,
    pub gid: u16,
    pub mode: u32,
    pub ctime: u32,
    pub mtime: u32,
    pub size: u32,
    pub pointers: [u32; NUM_POINTERS],
}

impl FSINode{
    pub fn to_fileattr(&self, node_num: u64) -> FileAttr {
        let dir_mask = 0o40000;

        let ftype = if self.mode & dir_mask != 0 { Directory } else { RegularFile };

        FileAttr{
            ino: node_num,
            uid: self.uid as u32,
            gid: self.gid as u32,
            mtime: crate::time_to_timespec(self.mtime),
            ctime: crate::time_to_timespec(self.ctime),
            crtime: crate::time_to_timespec(self.ctime),
            atime: crate::time_to_timespec(self.mtime),
            size: self.size as u64,
            blocks: self.pointers.clone().into_iter().filter(|ptr| *ptr != 0).sum::<u32>() as u64, // Because the file might be sparse
            nlink: 1,
            rdev: 0,
            flags: 0,
            kind: ftype,
            perm: self.mode as u16
        }
    }
}


impl From<&[u8]> for FSINode {
    fn from(inode_bytes: &[u8]) -> Self {
        let uid =  u16::from_le_bytes(crate::slice_to_two_bytes(&inode_bytes[0..2]));
        let gid =  u16::from_le_bytes(crate::slice_to_two_bytes(&inode_bytes[2..4]));
        let mode = u32::from_le_bytes(crate::slice_to_four_bytes(&inode_bytes[4..8]));
        let ctime = u32::from_le_bytes(crate::slice_to_four_bytes(&inode_bytes[8..12]));
        let mtime = u32::from_le_bytes(crate::slice_to_four_bytes(&inode_bytes[12..16]));
        let size = u32::from_le_bytes(crate::slice_to_four_bytes(&inode_bytes[16..20]));


        let mut pointers = [0u32; NUM_POINTERS];
        let pointer_vec: Vec<u32> = inode_bytes[20..].chunks_exact(4)
            .map(|chunk| u32::from_le_bytes(crate::slice_to_four_bytes(chunk)))
            .collect();

        pointers.copy_from_slice(&pointer_vec);

        FSINode{
            uid, gid, mode, ctime, mtime, size, pointers,
        }
    }
}

impl Into<Vec<u8>> for FSINode{
    fn into(self) -> Vec<u8> {
        let mut dest = vec![0u8; FS_BLOCK_SIZE];
        dest[0..2].copy_from_slice(&self.uid.to_le_bytes());
        dest[2..4].copy_from_slice(&self.gid.to_le_bytes());
        dest[4..8].copy_from_slice(&self.mode.to_le_bytes());
        dest[8..12].copy_from_slice(&self.ctime.to_le_bytes());
        dest[12..16].copy_from_slice(&self.mtime.to_le_bytes());
        dest[16..20].copy_from_slice(&self.size.to_le_bytes());

        for (ptr_idx, ptr_val) in self.pointers.iter().enumerate() {
            let dest_idx = 20 + (ptr_idx * 4);
            dest[dest_idx..(dest_idx+4)].copy_from_slice(&ptr_val.to_le_bytes());
        }

        dest
    }
}
