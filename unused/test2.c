/*
 * file:        unittest-2.c
 * description: libcheck test skeleton, part 2
 */

#define _FILE_OFFSET_BITS 64
#define FUSE_USE_VERSION 26

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <check.h>
#include <zlib.h>
#include <fuse.h>
#include <stdlib.h>
#include <errno.h>

extern struct fuse_operations fs_ops;
extern void block_init(char *file);

/* mockup for fuse_get_context. you can change ctx.uid, ctx.gid in
 * tests if you want to test setting UIDs in mknod/mkdir
 */
struct fuse_context ctx = { .uid = 501, .gid = 502};
struct fuse_context *fuse_get_context(void)
{
    return &ctx;
}

void new_image(void)
{
    system("python2 gen-disk.py -q disk2.in test2.img");
    block_init("test2.img");
    fs_ops.init(NULL);
}

int mkdir_1_filler(void *ptr, const char *name,
                   const struct stat *stbuf, off_t off)
{
    const char **list = ptr;
    for (int i = 0; ; i++)
        if (list[i] == NULL) {
            list[i] = strdup(name);
            break;
        }
    return 0;
}


/*
 * unit test: mkdir_0
 */
START_TEST(mkdir_0)
{
    new_image();
    int rv = 0;
    char *dirs[] = {"/dir1", "/dir2", "/dir3", "/dir1/dir4",
                    "/dir1/dir5", 0};
    for (int i = 0; dirs[i] != NULL; i++) {
        rv = fs_ops.mkdir(dirs[i], 0777);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(dirs[i], &sb);
        ck_assert_int_eq(rv, 0);
    }

    char *root[] = {NULL, NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", root, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(root[0], "dir1");
    ck_assert_str_eq(root[1], "dir2");
    ck_assert_str_eq(root[2], "dir3");
    ck_assert_ptr_eq(root[3], NULL);

    char *d1[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir1", d1, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d1[0], "dir4");
    ck_assert_str_eq(d1[1], "dir5");
    ck_assert_ptr_eq(d1[2], NULL);

    rv = fs_ops.create("/dir1/filexxx", 0777, NULL);
    ck_assert_int_eq(rv, 0);
    struct stat sb;
    rv = fs_ops.getattr("/dir1/filexxx", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0777);
    ck_assert_int_eq(sb.st_size, 0);

    rv = fs_ops.create("/dir1/dir4/file1", 0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.getattr("/dir1/dir4/file1", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0777);
    ck_assert_int_eq(sb.st_size, 0);
}
END_TEST

START_TEST(mkdir_time)
{
    new_image();
    int rv = 0;
    char *dirs[] = {"/dir1", 0};
    for (int i = 0; dirs[i] != NULL; i++) {
        uint32_t now = time(NULL);
        rv = fs_ops.mkdir(dirs[i], 0777);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(dirs[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_ge(sb.st_ctime, now);
        ck_assert_int_lt(sb.st_ctime, now+10); // 10 sec
    }
}
END_TEST

/* - create multiple subdirectories (with `mkdir`) in a directory, verify that they show up in `readdir`
   - delete them with `rmdir`, verify they don't show up anymore
Now run these tests (a) in the root directory, (b) in a subdirectory (e.g. "/dir1") and (c) in a sub-sub-directory (e.g. "/dir1/dir2")
*/

START_TEST(mkdir_rmdir_1)
{
    struct statvfs sv;
    new_image();
    int rv = fs_ops.statfs("/", &sv);
    ck_assert_int_eq(rv, 0);
    int blks = sv.f_bfree;

    char *dirs[] = {"/dir1", "/dir2", "/dir3", "/dir1/dir4",
                    "/dir1/dir5", "/dir1/dir4/dir6", 0};
    for (int i = 0; dirs[i] != NULL; i++) {
        rv = fs_ops.mkdir(dirs[i], 0777);
        ck_assert_int_eq(rv, 0);
    }

    char *root[] = {NULL, NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", root, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(root[0], "dir1");
    ck_assert_str_eq(root[1], "dir2");
    ck_assert_str_eq(root[2], "dir3");
    ck_assert_ptr_eq(root[3], NULL);

    char *d1[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir1", d1, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d1[0], "dir4");
    ck_assert_str_eq(d1[1], "dir5");
    ck_assert_ptr_eq(d1[2], NULL);

    char *d2[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir2", d2, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_ptr_eq(d2[0], NULL);

    rv = fs_ops.readdir("/dir3", d2, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_ptr_eq(d2[0], NULL);

    char *d4[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir1/dir4", d4, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d4[0], "dir6");
    ck_assert_ptr_eq(d4[1], NULL);

    fs_ops.rmdir("/dir1/dir5");
    ck_assert_int_eq(rv, 0);
    fs_ops.rmdir("/dir1/dir4/dir6");
    ck_assert_int_eq(rv, 0);
    fs_ops.rmdir("/dir1/dir4");
    ck_assert_int_eq(rv, 0);

    char *d5[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir1", d5, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_ptr_eq(d5[0], NULL);

    char *d6[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", d6, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d6[0], "dir1");
    ck_assert_str_eq(d6[1], "dir2");
    ck_assert_str_eq(d6[2], "dir3");
    ck_assert_ptr_eq(d6[3], NULL);

    fs_ops.rmdir("/dir1");
    ck_assert_int_eq(rv, 0);
    fs_ops.rmdir("/dir2");
    ck_assert_int_eq(rv, 0);
    fs_ops.rmdir("/dir3");
    ck_assert_int_eq(rv, 0);

    char *d7[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", d7, mkdir_1_filler, 0, NULL);
    ck_assert_ptr_eq(d7[0], NULL);

    rv = fs_ops.statfs("/", &sv);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(blks, sv.f_bfree);
}
END_TEST

int start_blocks(void)
{
    struct statvfs sv;
    int rv = fs_ops.statfs("/", &sv);
    ck_assert_int_eq(rv, 0);
    return sv.f_bfree;
}

void check_blocks(int blks)
{
    struct statvfs sv;
    int rv = fs_ops.statfs("/", &sv);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(blks, sv.f_bfree);
}

/* create directories, verify that owner & mode are correct */

START_TEST(mkdir_rmdir_2)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.mkdir("/dir10", 0765);
    ck_assert_int_eq(rv, 0);
    struct stat sb;
    rv = fs_ops.getattr("/dir10", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, 0040765);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);


    rv = fs_ops.mkdir("/dir10/dir11", 0640);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.getattr("/dir10/dir11", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, 0040640);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);

    rv = fs_ops.rmdir("/dir10/dir11");
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.rmdir("/dir10");
    ck_assert_int_eq(rv, 0);

    check_blocks(blks);
}
END_TEST


/*
 * Unit tests: create_0
 */

START_TEST(create_0)
{
    new_image();
    int rv;

    char *files1[] = {"/f1", "/f2", 0};
    for (int i = 0; files1[i]; i++) {
        rv = fs_ops.create(files1[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(files1[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_eq(sb.st_mode, S_IFREG|0777);
        ck_assert_int_eq(sb.st_size, 0);
    }

    char *d1[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", d1, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d1[0], "f1");
    ck_assert_str_eq(d1[1], "f2");
    ck_assert_ptr_eq(d1[2], NULL);
}
END_TEST

START_TEST(create_time)
{
    new_image();
    int rv;

    char *files1[] = {"/f1", 0};
    for (int i = 0; files1[i]; i++) {
        uint32_t now = time(NULL);
        rv = fs_ops.create(files1[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(files1[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_ge(sb.st_ctime, now);
        ck_assert_int_lt(sb.st_ctime, now+10); // 10 sec
    }
}
END_TEST

/*
 * Unit tests: unlink_0
 */

START_TEST(unlink_0)
{
    new_image();
    int blks = start_blocks();
    int rv;

    char *files1[] = {"/f1", "/f2", 0};
    for (int i = 0; files1[i]; i++) {
        rv = fs_ops.create(files1[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);
    }
    char *d1[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", d1, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d1[0], "f1");
    ck_assert_str_eq(d1[1], "f2");
    ck_assert_ptr_eq(d1[2], NULL);

    for (int i = 0; files1[i]; i++) {
        rv = fs_ops.unlink(files1[i]);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(files1[i], &sb);
        ck_assert_int_lt(rv, 0);

        for (int j=i+1; files1[j]; j++){
            rv = fs_ops.getattr(files1[j], &sb);
            ck_assert_int_eq(rv, 0);
        }
    }

    check_blocks(blks);
}
END_TEST


START_TEST(unlink_time)
{
    new_image();
    int rv;

    rv = fs_ops.mkdir("/dir1", 0777);
    ck_assert_int_eq(rv, 0);

    rv = fs_ops.create("/dir1/file1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

    struct stat sb;
    rv = fs_ops.getattr("/dir1/file1", &sb);
    ck_assert_int_eq(rv, 0);

    printf("  sleep 1 sec\n");
    sleep(1);

    uint32_t now = time(NULL);
    rv = fs_ops.unlink("/dir1/file1");
    ck_assert_int_eq(rv, 0);

    rv = fs_ops.getattr("/dir1/", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_ge(sb.st_mtime, now);
    ck_assert_int_lt(sb.st_mtime, now + 10); // 10 sec
}
END_TEST


/* - create multiple files in a directory (with `create`), verify that they show up in `readdir`
   - delete them (`unlink`), verify they don't show up anymore */

START_TEST(create_mkdir_unlink_rmdir_1)
{
    new_image();
    int blks = start_blocks();
    int rv;

    char *files1[] = {"/f1", "/f2", 0};
    for (int i = 0; files1[i]; i++) {
        rv = fs_ops.create(files1[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);
    }
    char *d1[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/", d1, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d1[0], "f1");
    ck_assert_str_eq(d1[1], "f2");
    ck_assert_ptr_eq(d1[2], NULL);

    rv = fs_ops.mkdir("/dir20", 0777);
    ck_assert_int_eq(rv, 0);
    char *files2[] = {"/dir20/f3", "/dir20/f4", 0};
    for (int i = 0; files2[i]; i++) {
        int rv = fs_ops.create(files2[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);


        struct stat sb;
        rv = fs_ops.getattr(files2[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_eq(sb.st_mode, S_IFREG|0777);
        ck_assert_int_eq(sb.st_size, 0);
    }
    char *d2[] = {NULL, NULL, NULL, NULL};
    rv = fs_ops.readdir("/dir20", d2, mkdir_1_filler, 0, NULL);
    ck_assert_int_eq(rv, 0);
    ck_assert_str_eq(d2[0], "f3");
    ck_assert_str_eq(d2[1], "f4");
    ck_assert_ptr_eq(d2[2], NULL);

    char *d3[] = {"/f1", "/f2", "/dir20/f3", "/dir20/f4", 0};
    for (int i = 0; d3[i]; i++) {
        rv = fs_ops.unlink(d3[i]);
        ck_assert_int_eq(rv, 0);
    }

    rv = fs_ops.rmdir("/dir20");
    ck_assert_int_eq(rv, 0);

    check_blocks(blks);
}
END_TEST

/* like mkdir, make sure stat info (uid, gid, mode, size) is correct */

START_TEST(create_unlink_2)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.create("/f5", S_IFREG|0765, NULL);
    ck_assert_int_eq(rv, 0);
    struct stat sb;
    rv = fs_ops.getattr("/f5", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0765);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);
    ck_assert_int_eq(sb.st_size, 0);


    rv = fs_ops.create("/f6", S_IFREG|0765, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.getattr("/f6", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0765);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);

    rv = fs_ops.unlink("/f5");
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.unlink("/f6");
    ck_assert_int_eq(rv, 0);

    check_blocks(blks);
}
END_TEST

/* Once you've implemented `write`, add an additional set of unlink tests where you write data into the files first. You might want to use `statvfs` to verify that unlink freed the blocks afterwards. (the grading tests will do that)  */

START_TEST(create_write_unlink_3)
{
    new_image();
    int blks = start_blocks();
    char buf[3000];

    int rv = fs_ops.create("/f5", S_IFREG|0765, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.write("/f5", buf, 3000, 0, NULL);
    ck_assert_int_eq(rv, 3000);

    struct stat sb;
    rv = fs_ops.getattr("/f5", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0765);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);
    ck_assert_int_eq(sb.st_size, 3000);

    rv = fs_ops.create("/f6", S_IFREG|0765, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.write("/f6", buf, 3000, 0, NULL);
    ck_assert_int_eq(rv, 3000);

    rv = fs_ops.getattr("/f6", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sb.st_mode, S_IFREG|0765);
    ck_assert_int_eq(sb.st_uid, 501);
    ck_assert_int_eq(sb.st_gid, 502);
    ck_assert_int_eq(sb.st_size, 3000);

    rv = fs_ops.unlink("/f5");
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.unlink("/f6");
    ck_assert_int_eq(rv, 0);

    check_blocks(blks);
}
END_TEST

START_TEST(mkdir_rmdir_unlink_fail)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.mkdir("/dir1", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2/dir3", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/file1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/file2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/dir2/file3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

    //-  bad path /a/b/c - b doesn't exist (ENOENT)
    rv = fs_ops.mkdir("/dir1/dir5/dir", 0777);
    ck_assert_int_eq(rv, -ENOENT);

    //-  bad path /a/b/c - b isn't directory (ENOTDIR)
    rv = fs_ops.mkdir("/dir1/file2/dir", 0777);
    ck_assert_int_eq(rv, -ENOTDIR);

    //-  bad path /a/b/c - c exists, is file (EEXIST)
    rv = fs_ops.mkdir("/dir1/dir2/file3", 0777);
    ck_assert_int_eq(rv, -EEXIST);

    //-  bad path /a/b/c - c exists, is directory (EEXIST)
    rv = fs_ops.mkdir("/dir1/dir2/dir3", 0777);
    ck_assert_int_eq(rv, -EEXIST);

    //-  too-long name
    rv = fs_ops.mkdir("/dir1/dir2/this-directory-is-really-long", 0777);
    ck_assert_int_eq(rv, -EINVAL);

    /* now let's try this all again, but in the root directory */

    //-  bad path /a/c - a doesn't exist (ENOENT)
    rv = fs_ops.mkdir("/dir7/dir", 0777);
    ck_assert_int_eq(rv, -ENOENT);

    //-  bad path /a/c - a isn't directory (ENOTDIR)
    rv = fs_ops.mkdir("/file1/dir", 0777);
    ck_assert_int_eq(rv, -ENOTDIR);

    //-  bad path /a/c - c exists, is file (EEXIST)
    rv = fs_ops.mkdir("/dir1/file2", 0777);
    ck_assert_int_eq(rv, -EEXIST);

    //-  bad path /a/c - c exists, is directory (EEXIST)
    rv = fs_ops.mkdir("/dir1/dir2", 0777);
    ck_assert_int_eq(rv, -EEXIST);

    //-  too-long name
    rv = fs_ops.mkdir("/this-directory-is-really-long", 0777);
    ck_assert_int_eq(rv, -EINVAL);

    char *dirs[] = {"/dir1/dir2/dir3", "/dir1/dir2", "/dir1", 0};
    char *files[] = {"/dir1/dir2/file3", "/dir1/file2", "/file1", 0};
    for (int i = 0; files[i]; i++) {
        rv = fs_ops.unlink(files[i]);
        ck_assert_int_eq(rv, 0);
    }
    for (int i = 0; dirs[i]; i++) {
        rv = fs_ops.rmdir(dirs[i]);
        ck_assert_int_eq(rv, 0);
    }

    check_blocks(blks);
}
END_TEST

START_TEST(create_fail)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.mkdir("/dir1", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2/dir3", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/file1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/file2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/dir2/file3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

    //-  bad path /a/b/c - b doesn't exist (ENOENT)
    rv = fs_ops.create("/dir1/dir5/file", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -ENOENT);

    //-  bad path /a/b/c - b isn't directory (ENOTDIR)
    rv = fs_ops.create("/dir1/file2/file", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -ENOTDIR);

    //-  bad path /a/b/c - c exists, is file (EEXIST)
    rv = fs_ops.create("/dir1/dir2/file3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EEXIST);

    //-  bad path /a/b/c - c exists, is directory (EEXIST)
    rv = fs_ops.create("/dir1/dir2/dir3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EEXIST);

    //-  too-long name
    rv = fs_ops.create("/dir1/dir2/this-filename-is-also-really-long", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EINVAL);

    /* now let's try this all again, but in the root directory */

    //-  bad path /a/c - a doesn't exist (ENOENT)
    rv = fs_ops.create("/dir7/file", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -ENOENT);

    //-  bad path /a/c - a isn't directory (ENOTDIR)
    rv = fs_ops.create("/file1/file", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -ENOTDIR);

    //-  bad path /a/c - c exists, is file (EEXIST)
    rv = fs_ops.create("/dir1/file2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EEXIST);

    //-  bad path /a/c - c exists, is directory (EEXIST)
    rv = fs_ops.create("/dir1/dir2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EEXIST);

    //-  too-long name
    rv = fs_ops.create("/this-filename-is-also-really-long", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, -EINVAL);

    char *dirs[] = {"/dir1/dir2/dir3", "/dir1/dir2", "/dir1", 0};
    char *files[] = {"/dir1/dir2/file3", "/dir1/file2", "/file1", 0};
    for (int i = 0; files[i]; i++) {
        rv = fs_ops.unlink(files[i]);
        ck_assert_int_eq(rv, 0);
    }
    for (int i = 0; dirs[i]; i++) {
        rv = fs_ops.rmdir(dirs[i]);
        ck_assert_int_eq(rv, 0);
    }

    check_blocks(blks);
}
END_TEST

START_TEST(unlink_fail)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.mkdir("/dir1", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2/dir3", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/file1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/file2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/dir2/file3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

//-  bad path /a/b/c - b doesn't exist (ENOENT)
    rv = fs_ops.unlink("/dir1/nothing/file3");
    ck_assert_int_eq(rv, -ENOENT);

//-  bad path /a/b/c - b isn't directory (ENOTDIR)
    rv = fs_ops.unlink("/dir1/file2/file3");
    ck_assert_int_eq(rv, -ENOTDIR);

//-  bad path /a/b/c - c doesn't exist (ENOENT)
    rv = fs_ops.unlink("/dir1/dir2/nothing");
    ck_assert_int_eq(rv, -ENOENT);

//-  bad path /a/b/c - c is directory (EISDIR)
    rv = fs_ops.unlink("/dir1/dir2/dir3");
    ck_assert_int_eq(rv, -EISDIR);

    char *dirs[] = {"/dir1/dir2/dir3", "/dir1/dir2", "/dir1", 0};
    char *files[] = {"/dir1/dir2/file3", "/dir1/file2", "/file1", 0};
    for (int i = 0; files[i]; i++) {
        rv = fs_ops.unlink(files[i]);
        ck_assert_int_eq(rv, 0);
    }
    for (int i = 0; dirs[i]; i++) {
        rv = fs_ops.rmdir(dirs[i]);
        ck_assert_int_eq(rv, 0);
    }

    check_blocks(blks);
}
END_TEST

START_TEST(rmdir_fail)
{
    new_image();
    int blks = start_blocks();

    int rv = fs_ops.mkdir("/dir1", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.mkdir("/dir1/dir2/dir3", 0777);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/file1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/file2", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    rv = fs_ops.create("/dir1/dir2/file3", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

//-  bad path /a/b/c - b doesn't exist (ENOENT)
    rv = fs_ops.rmdir("/dir1/nothing/dir3");
    ck_assert_int_eq(rv, -ENOENT);

//-  bad path /a/b/c - b isn't directory (ENOTDIR)
    rv = fs_ops.rmdir("/dir1/file2/dir3");
    ck_assert_int_eq(rv, -ENOTDIR);

//-  bad path /a/b/c - c doesn't exist (ENOENT)
    rv = fs_ops.rmdir("/dir1/dir2/nothing");
    ck_assert_int_eq(rv, -ENOENT);

//-  bad path /a/b/c - c is file (ENOTDIR)
    rv = fs_ops.rmdir("/dir1/dir2/file3");
    ck_assert_int_eq(rv, -ENOTDIR);

//-  bad path /a/b/c - c not empty (ENOTEMPTY)
    rv = fs_ops.rmdir("/dir1/dir2");
    ck_assert_int_eq(rv, -ENOTEMPTY);

    char *dirs[] = {"/dir1/dir2/dir3", "/dir1/dir2", "/dir1", 0};
    char *files[] = {"/dir1/dir2/file3", "/dir1/file2", "/file1", 0};
    for (int i = 0; files[i]; i++) {
        rv = fs_ops.unlink(files[i]);
        ck_assert_int_eq(rv, 0);
    }
    for (int i = 0; dirs[i]; i++) {
        rv = fs_ops.rmdir(dirs[i]);
        ck_assert_int_eq(rv, 0);
    }

    check_blocks(blks);
}
END_TEST

void *rnd_data(size_t len)
{
    unsigned char *p = malloc(len+10);
    for (int i = 0; i < len+10; i++)
        p[i] = random();
    return p;
}

int write_lens[] = {17, 1024, 4000, 4096, 5000};

void _do_write(char *path, void *ptr, int len, int blksiz)
{
    int offset = 0;
    while (len > 0) {
        int bytes = (len > blksiz) ? blksiz : len;
        int rv = fs_ops.write(path, ptr, bytes, offset, NULL);
        ck_assert_int_eq(rv, bytes);
        ptr += bytes;
        offset += bytes;
        len -= bytes;
    }
}

void *read_back(char *path, int len)
{
    void *p = malloc(len+50);
    struct stat sb;
    int rv = fs_ops.getattr(path, &sb);
    ck_assert(S_ISREG(sb.st_mode));
    rv = fs_ops.read(path, p, len+50, 0, NULL);
    ck_assert_int_eq(rv, len);
    return p;
}

void do_write_test(int file_len)
{
    void *data_in = rnd_data(file_len);

    int rv;
    struct {char *path; int len;} tests[] = {
        {"/file1", 1},
        {"/file2", 17},
        {"/file3", 4001},
        {"/file4", 4096},
        {"/file5", 5001},
        {"/file6", 100000},
        {NULL, 0}};
    char *path;

    for (int i = 0; (path = tests[i].path) != NULL; i++) {
        int len = tests[i].len;
        rv = fs_ops.create(path, S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);
        printf("  file size: %d block size: %d\n", file_len, len);
        _do_write(path, data_in, file_len, len);
        void *data_out = read_back(path, file_len);
        ck_assert(memcmp(data_in, data_out, file_len) == 0);
        free(data_out);
    }
    free(data_in);

    for (int i = 0; (path = tests[i].path) != NULL; i++) {
        rv = fs_ops.unlink(path);
        ck_assert_int_eq(rv, 0);
    }
}

START_TEST(write_4091)
{
    new_image();
    int blks = start_blocks();
    do_write_test(4091);
    check_blocks(blks);
}
END_TEST

START_TEST(write_time)
{
    new_image();
    int rv;

    rv = fs_ops.create("/f1", S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);

    struct stat sb;
    rv = fs_ops.getattr("/f1", &sb);
    ck_assert_int_eq(rv, 0);

    printf("  sleep 1 sec\n");
    sleep(1);

    uint32_t now = time(NULL);
    void *data_in = rnd_data(10);
    _do_write("/f1", data_in, 10, 1);
    free(data_in);

    rv = fs_ops.getattr("/f1", &sb);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_ge(sb.st_mtime, now);
    ck_assert_int_lt(sb.st_mtime, now + 10); // 10 sec

}
END_TEST

START_TEST(write_4096)
{
    new_image();
    int blks = start_blocks();
    do_write_test(4096);
    check_blocks(blks);
}
END_TEST

START_TEST(write_4200)
{
    new_image();
    int blks = start_blocks();
    do_write_test(4200);
    check_blocks(blks);
}
END_TEST

START_TEST(write_8190)
{
    new_image();
    int blks = start_blocks();
    do_write_test(8190);
    check_blocks(blks);
}
END_TEST

START_TEST(write_12300)
{
    new_image();
    int blks = start_blocks();
    do_write_test(12300);
    check_blocks(blks);
}
END_TEST

/* @size - file length to create
 * @offset, @len - for overwrite
 */
void do_overwrite_test(char *path, size_t size, size_t offset, size_t len)
{
    int rv = fs_ops.create(path, S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    void *data_1 = rnd_data(size);
    _do_write(path, data_1, size, 10*4096);

    void *data_2 = rnd_data(len);
    rv = fs_ops.write(path, data_2, len, offset, NULL);
    ck_assert_int_eq(rv, len);

    memcpy(data_1+offset, data_2, len);
    void *data_3 = read_back(path, size);

    ck_assert(memcmp(data_1, data_3, size) == 0);
    free(data_1);
    free(data_2);
    free(data_3);

    rv = fs_ops.unlink(path);
    ck_assert_int_eq(rv, 0);
}

// single-block file, overwrite within block
START_TEST(overwrite_1)
{
    new_image();
    int blks = start_blocks();
    do_overwrite_test("/file7", 4000, 1000, 1200);
    check_blocks(blks);
}
END_TEST

// two-block file, cross the boundary
START_TEST(overwrite_2)
{
    new_image();
    int blks = start_blocks();
    do_overwrite_test("/file7", 6000, 1000, 4000);
    check_blocks(blks);
}
END_TEST

// four-block file, cross multiple boundaries
START_TEST(overwrite_3)
{
    new_image();
    int blks = start_blocks();
    do_overwrite_test("/file7", 12000, 1000, 9000);
    check_blocks(blks);
}
END_TEST

START_TEST(cross_eof)
{
    new_image();
    int blks = start_blocks();
    char *path = "/file17";

    int rv = fs_ops.create(path, S_IFREG|0777, NULL);
    ck_assert_int_eq(rv, 0);
    void *data_1 = rnd_data(8000);
    _do_write(path, data_1, 8000, 10*4096);

    // first 5000 from data_1, next 7000 from data_2
    void *data_2 = rnd_data(7000);
    rv = fs_ops.write(path, data_2, 7000, 5000, NULL);
    ck_assert_int_eq(rv, 7000);

    void *data_3 = read_back(path, 12000);
    ck_assert(memcmp(data_1, data_3, 5000) == 0);
    ck_assert(memcmp(data_2, data_3+5000, 7000) == 0);

    free(data_1);
    free(data_2);
    free(data_3);

    rv = fs_ops.unlink(path);
    ck_assert_int_eq(rv, 0);

    check_blocks(blks);
}
END_TEST

START_TEST(truncate_test)
{
    new_image();
    int blks = start_blocks();
    struct stat sb;
    int sizes[] = {4000, 4096, 8000, 8192, 12288, 13000, 0};

    for (int i = 0; sizes[i] != 0; i++) {
        char path[32];
        sprintf(path, "/file_%d", sizes[i]);
        int rv = fs_ops.create(path, S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);
        void *data_1 = rnd_data(sizes[i]);
        _do_write(path, data_1, sizes[i], 10*4096);
        free(data_1);
        rv = fs_ops.truncate(path, 0);
        ck_assert_int_eq(rv, 0);
        rv = fs_ops.getattr(path, &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_eq(sb.st_size, 0);
    }

    for (int i = 0; sizes[i] != 0; i++) {
        char path[32];
        sprintf(path, "/file_%d", sizes[i]);
        int rv = fs_ops.unlink(path);
        ck_assert_int_eq(rv, 0);
    }
    check_blocks(blks);
}
END_TEST


START_TEST(utime_0)
{
    new_image();
    int rv;

    char *files1[] = {"/f1", "/f2", 0};
    for (int i = 0; files1[i]; i++) {
        rv = fs_ops.create(files1[i], S_IFREG|0777, NULL);
        ck_assert_int_eq(rv, 0);

        uint32_t now = time(NULL);
        rv = fs_ops.utime(files1[i], NULL);
        ck_assert_int_eq(rv, 0);

        struct stat sb;
        rv = fs_ops.getattr(files1[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_ge(sb.st_mtime, now);
        ck_assert_int_lt(sb.st_mtime, now+10);  // within 10sec


        // NULL
        struct utimbuf ut;
        ut.modtime = time(NULL) + 20;
        rv = fs_ops.utime(files1[i], &ut);
        ck_assert_int_eq(rv, 0);

        rv = fs_ops.getattr(files1[i], &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_eq(sb.st_mtime, ut.modtime);
    }

}
END_TEST


int lengths[] = {1, 117, 4091, 4096, 5010, 8190, 8192, 8300, 12200, 12288, 12300, 0};



extern struct fuse_operations fs_ops;
extern void block_init(char *file);

int main(int argc, char **argv)
{

    Suite *s = suite_create("fs5600");
    TCase *tc = tcase_create("write_mostly");

    tcase_add_test(tc, create_0);
    tcase_add_test(tc, create_time);
    tcase_add_test(tc, mkdir_0);
    tcase_add_test(tc, mkdir_time);
    tcase_add_test(tc, unlink_0);
    tcase_add_test(tc, unlink_time);

    tcase_add_test(tc, mkdir_rmdir_1);
    tcase_add_test(tc, mkdir_rmdir_2);
    tcase_add_test(tc, mkdir_rmdir_unlink_fail);
    tcase_add_test(tc, create_mkdir_unlink_rmdir_1);
    tcase_add_test(tc, create_unlink_2);
    tcase_add_test(tc, create_fail);
    tcase_add_test(tc, unlink_fail);
    tcase_add_test(tc, rmdir_fail);
    tcase_add_test(tc, write_4091);
    tcase_add_test(tc, write_time);
    tcase_add_test(tc, write_4096);
    tcase_add_test(tc, write_4200);
    tcase_add_test(tc, write_8190);
    tcase_add_test(tc, write_12300);
    tcase_add_test(tc, create_write_unlink_3);
    tcase_add_test(tc, overwrite_1);
    tcase_add_test(tc, overwrite_2);
    tcase_add_test(tc, overwrite_3);
    tcase_add_test(tc, cross_eof);
    tcase_add_test(tc, truncate_test);
    tcase_add_test(tc, utime_0);

    suite_add_tcase(s, tc);
    SRunner *sr = srunner_create(s);
//    srunner_set_fork_status(sr, CK_NOFORK);

    srunner_run_all(sr, CK_VERBOSE);
    int n_failed = srunner_ntests_failed(sr);
    printf("%d tests failed\n", n_failed);

    srunner_free(sr);
    return (n_failed == 0) ? EXIT_SUCCESS : EXIT_FAILURE;
}

