/*
 * file:        fsx600.h
 * description: Data structures for CS 5600/7600 file system.
 *
 * CS 5600, Computer Systems, Northeastern CCIS
 * Peter Desnoyers,  November 2016
 *
 * Modified by CS5600 staff in fall 2021
 */
#ifndef __CSX600_H__
#define __CSX600_H__

#define FS_BLOCK_SIZE 4096
#define FS_MAGIC 0x30303635

/* how many buckets of size M do you need to hold N items?
 */
#define DIV_ROUND_UP(N, M) ((N) + (M) - 1) / (M)

/*
 * number of pointers in one inode
 */
#define NUM_PTRS_INODE (FS_BLOCK_SIZE/4 - 5)

/*
 * number of directory entries (dirent_t) in one block
 */
#define NUM_DIRENT_BLOCK (FS_BLOCK_SIZE / sizeof(struct fs_dirent))

/* Superblock - holds file system parameters.
 */
struct fs_super {
    uint32_t magic;
    uint32_t disk_size;         /* in blocks */

    /* pad out to an entire block */
    char pad[FS_BLOCK_SIZE - 2 * sizeof(uint32_t)];
};

struct fs_inode {
    uint16_t uid;
    uint16_t gid;
    uint32_t mode;
    uint32_t ctime;     // time of last status change; see more in "man 2 stat"
    uint32_t mtime;     // last modification time
    int32_t  size;
    uint32_t ptrs[NUM_PTRS_INODE]; /* inode = 4096 bytes */
};

/* Entry in a directory
 */
struct fs_dirent {
    uint32_t valid : 1;
    uint32_t inode : 31;
    char name[28];              /* with trailing NUL */
};


typedef struct fs_super super_t;
typedef struct fs_inode inode_t;
typedef struct fs_dirent dirent_t;

#endif
