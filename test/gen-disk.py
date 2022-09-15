#!/usr/bin/python
#
# usage: gen-disk2.py input output.img
#
# see comments in disk1.in for file format

import sys
import diskfmt as fs
import random as rnd

quiet = False
if sys.argv[1] == '-q':
    quiet = True
    sys.argv.pop(1)

chars = 'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ'

class file(object):
    def __init__(self, fields):
        inum,self.name,self.uid,self.gid,self.mode,self.ctime,self.mtime,size,blocks = fields
        self.inum = int(inum)
        self.size = int(size)
        self.blocks = map(int, blocks.split(','))

    def inode(self):
        i = fs.inode()
        i.uid, i.gid, i.mode = self.uid, self.gid, self.mode
        i.ctime, i.mtime, i.size = self.ctime, self.mtime, self.size
        for j in range(len(self.blocks)):
            i.ptrs[j] = self.blocks[j]
        return bytearray(i)

    def block(self,offset):
        if not quiet:
            print("block", self.name, offset)
        rnd.seed(hash(self.name) + offset)
        val = ''
        for i in range(4096):
            n = rnd.randint(0,50)
            val = val + chars[n]
        return bytearray(val)
        
class dir(object):
    def __init__(self, fields):
        self.inum = 0
        inum,self.name,self.uid,self.gid,self.mode,self.ctime,self.mtime,size,blocks = fields[0:9]
        self.inum = int(inum)
        self.size = int(size)
        self.blocks = map(int, blocks.split(','))
        entries = fields[9:]
        self.entries = []
        for e in fields[9:]:
            if e[0] == '-':
                self.entries.append([False, e[1:], 0])
            else:
                name,inum = e.split(',')
                self.entries.append([True, name, int(inum)])

    def inode(self):
        i = fs.inode()
        i.uid, i.gid, i.mode = self.uid, self.gid, self.mode
        i.ctime, i.mtime, i.size = self.ctime, self.mtime, self.size
        for j in range(len(self.blocks)):
            i.ptrs[j] = self.blocks[j]
        return bytearray(i)

    # dirent is 32 bytes, 128 per block
    def block(self,offset):
        data = bytearray(4096)
        de = fs.dirent()
        j = 0
        for i in range(offset*128, min(len(self.entries), (offset+1)*128)):
            val,name,num = self.entries[i]
            de.valid, de.inode, de.name = val, num, name
            data[j:j+32] = bytearray(de)
            j += 32
        return data
        
        
syms = dict()
files = []
dirs = []
nblocks = 0
magic = 0x30303635

for line in open(sys.argv[1],'r'):
    fields = line.split()

    if line[0] == '#' or len(fields) == 0:
        continue

    if line[0] == '$':
        syms[fields[0]] = int(fields[1],0)
        continue

    if fields[0] == 'size':
        nblocks = int(fields[1])
        continue
    
    for i in range(len(fields)):
        if fields[i][0] == '$':
            fields[i] = syms[fields[i]]
    if fields[0] == 'file':
        files.append(file(fields[1:]))
    if fields[0] == 'dir':
        dirs.append(dir(fields[1:]))

blockmap = fs.bitmap()
blockmap.set(0,True)                      # superblock
blockmap.set(1,True)                      # bitmap

blocks = [None] * 400

for f in files + dirs:
    blocks[f.inum] = [f]
    blockmap.set(f.inum, True)
    i = 0
    for b in f.blocks:
        if blockmap.get(b):
            print('ERROR: double counted', b)
        blockmap.set(b, True)
        blocks[b] = [f,i]
        i += 1

sb = fs.super()
sb.magic, sb.disk_sz = magic, nblocks
zeros = bytearray(4096)

fp = open(sys.argv[2], 'wb')
fp.write(bytearray(sb))
fp.write(bytearray(blockmap))
for i in range(2,nblocks):
    if not blocks[i]:
        fp.write(zeros)
    elif len(blocks[i]) == 1:
        inode = blocks[i][0]
        fp.write(inode.inode())
    else:
        item,offset = blocks[i]
        if not quiet:
            print('item', item.name)
        fp.write(item.block(offset))
fp.close()


