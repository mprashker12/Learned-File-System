/*
 * file:        misc.c
 * description: various support functions for CS 5600/7600 file system
 *              startup argument parsing and checking, etc.
 *
 * CS 5600, Computer Systems, Northeastern CCIS
 * Peter Desnoyers, November 2016
 */

#define _XOPEN_SOURCE 500
#define _FILE_OFFSET_BITS 64

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <string.h>
#include <errno.h>
#include <stdint.h>
#include <fcntl.h>
#include <assert.h>

#include "fs5600.h"        /* only for FS_BLOCK_SIZE */

/*********** DO NOT MODIFY THIS FILE *************/

/* All disk I/O is accessed through these functions
 */
static int disk_fd;

/* read blocks from disk image. Returns -EIO if error, 0 otherwise
 */
int block_read(char *buf, int lba, int nblks)
{
    int len = nblks * FS_BLOCK_SIZE, start = lba * FS_BLOCK_SIZE;

    if (lseek(disk_fd, start, SEEK_SET) < 0)
        return -EIO;
    if (read(disk_fd, buf, len) != len)
        return -EIO;
    return 0;
}

/* write blocks from disk image. Returns -EIO if error, 0 otherwise
 */
int block_write(char *buf, int lba, int nblks)
{
    int len = nblks * FS_BLOCK_SIZE, start = lba * FS_BLOCK_SIZE;

    assert(lba > 0);        /* write to 0 is *always* an error */

    /* Seek to the *end* of the region being written, to make sure it
     * all fits on the disk image. Then seek to write location.
     */
    if (lseek(disk_fd, start + len, SEEK_SET) < 0)
        return -EIO;
    if (lseek(disk_fd, start, SEEK_SET) < 0)
        return -EIO;
    if (write(disk_fd, buf, len) != len)
        return -EIO;
    return 0;
}

void block_init(char *file)
{
    if (strlen(file) < 4 || strcmp(file+strlen(file)-4, ".img") != 0) {
        printf("bad image file (must end in .img): %s\n", file);
        exit(1);
    }
    if ((disk_fd = open(file, O_RDWR)) < 0) {
        printf("cannot open image file '%s': %s\n", file, strerror(errno));
        exit(1);
    }
}
