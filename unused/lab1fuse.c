/*
 * file:        lab1fuse.c
 * description: main() for fs5600 in FUSE mode
 */
#define FUSE_USE_VERSION 27
#define _FILE_OFFSET_BITS 64

#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
#include <fuse.h>

#include "fs5600.h"

extern void block_init(char *file);

/* All fs5600 functions are accessed through the operations
 * structure.
 */
extern struct fuse_operations fs_ops;

struct data {
    char *image_name;
    int   part;
    int   cmd_mode;
} _data;

/**************/

/*
 * See comments in /usr/include/fuse/fuse_opts.h for details of
 * FUSE argument processing.
 *
 *  usage: ./lab1fuse -image disk.img directory
 *              disk.img  - name of the image file to mount
 *              directory - directory to mount it on
 */
static struct fuse_opt opts[] = {
    {"-image %s", offsetof(struct data, image_name), 0},
    FUSE_OPT_END
};

void usage(){
    printf("usage: ./lab1fuse -image disk.img directory \n");
    printf("             disk.img  - name of the image file to mount\n");
    printf("             directory - directory to mount it on\n");
}

int main(int argc, char **argv)
{
    /* Argument processing and checking
     */
    if (argc != 4) {
        usage();
        exit(1);
    }

    struct fuse_args args = FUSE_ARGS_INIT(argc, argv);
    if (fuse_opt_parse(&args, &_data, opts, NULL) == -1) {
        usage();
        exit(1);
    }

    block_init(_data.image_name);

    return fuse_main(args.argc, args.argv, &fs_ops, NULL);
}
