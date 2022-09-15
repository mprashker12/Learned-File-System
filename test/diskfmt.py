from ctypes import *

MAGIC = 0x30303635

class dirent(Structure):
    _fields_ = [("valid", c_uint, 1),
                ("inode", c_uint, 31),
                ("name", c_char * 28)]
        
class super(Structure):
    _fields_ = [("magic", c_uint),
                ("disk_sz", c_uint),
                ("_pad", c_char * 4088)]

class inode(Structure):
    _fields_ = [("uid", c_ushort),
                ("gid", c_ushort),
                ("mode", c_uint),
                ("ctime", c_uint),
                ("mtime", c_uint),
                ("size", c_int),
                ("ptrs", c_uint * 1019)]

class bitmap(Structure):
    _fields_ = [("vals", c_uint * 1024)]
    def get(self, i):
        n = self.vals[i // 32]
        mask = 1 << (i % 32)
        return (n & mask) != 0
    def set(self, i, val):
        mask = 1 << (i % 32)
        n = self.vals[i // 32]
        if val:
            n = n | mask
        else:
            n = n & (mask ^ 0xffffffff)
        self.vals[i // 32] = n

S_IFMT  = 0o0170000  # bit mask for the file type bit field
S_IFREG = 0o0100000  # regular file
S_IFDIR = 0o0040000  # directory

def S_ISREG(mode):
    return (mode & S_IFMT) == S_IFREG

def S_ISDIR(mode):
    return (mode & S_IFMT) == S_IFDIR
