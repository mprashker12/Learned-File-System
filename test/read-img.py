#!/usr/bin/python
import sys
import os
import diskfmt as fs

fd = os.open(sys.argv[1], os.O_RDONLY)
nbytes = os.fstat(fd).st_size
if nbytes % 4096 != 0:
    print 'BAD LENGTH: %d (0x%x)' % (nbytes, nbytes)
    sys.exit(1)

nblks = nbytes // 4096
blks = [bytes(os.read(fd, 4096)) for _ in range(nblks)]
sb = fs.super.from_buffer_copy(blks[0])
print ('superblock: magic:  %08X%s' %
           (sb.magic, ' *BAD*' if sb.magic != fs.MAGIC else ''))
print ('            blocks: %d%s' %
           (sb.disk_sz, (' *BAD* %d' % nblks) if sb.disk_sz != nblks else ''))
print

blkmap = fs.bitmap.from_buffer_copy(blks[1])
inodes = dict()

print("blocks used:"),
n,e = 0,''
for i in range(nblks):
    if blkmap.get(i):
        n += 1
        if n == 16:
            n,e = 0,('\n' + ' '*12)
        print ' %d%s' % (i, e),
        e = ''
print '\n'

names = dict()
names[2] = ''

def iter(name, inum, v):
    assert inum < nblks
    children = []
    inodes[inum] = 1
    _in = fs.inode.from_buffer_copy(blks[inum])
    alloc = '' if blkmap.get(inum) else 'NOT MARKED IN BITMAP '
    s = '/' if name is '' else name

    if v:
        print 'inode %d:' % inum
        print '  "%s" (%d,%d) %03o %d %s' % (s, _in.uid, _in.gid, _in.mode,
                                                 _in.size, alloc)
    
    xblks = (_in.size + 4095) // 4096
    if fs.S_ISREG(_in.mode):
        if v:
            print '  blocks: ',
        for i in range(xblks):
            alloc = '' if blkmap.get(_in.ptrs[i]) else '(NOT ALLOCATED)'
            if v:
                print str(_in.ptrs[i]) + alloc,
        if v:
            print
    elif fs.S_ISDIR(_in.mode):
        for i in range(xblks):
            dblk = _in.ptrs[i]
            alloc = '' if blkmap.get(_in.ptrs[i]) else '(NOT ALLOCATED)'
            if v:
                print '  block', dblk, alloc
            _blk = blks[dblk]
            des = [fs.dirent.from_buffer_copy(_blk[j:j+32])
                       for j in range(0, 4096, 32)]
            for j in range(128):
                if des[j].valid:
                    if v:
                        print '    [%d] "%s" -> %d' % (j, des[j].name, des[j].inode)
                    children.append([name + '/' + des[j].name, des[j].inode])
    else:
        if v:
            print 'Bad mode: %o' % _in.mode

    if v:
        print
    
    for n,i in children:
        iter(n,i, v)

iter('', 2, False)

print "inodes found:",

n,e = 0,''
for i in range(nblks):
    if i in inodes:
        n += 1
        if n == 16:
            n,e = 0,('\n' + ' '*12)
        print ' %d%s' % (i, e),
        e = ''
print '\n'

iter('', 2, True)

