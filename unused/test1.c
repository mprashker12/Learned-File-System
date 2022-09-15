/*
 * file:        test-q1.c
 * description: grading tests for part 1, 5600 fall 2020 project 3
 */

#define _FILE_OFFSET_BITS 64
#define FUSE_USE_VERSION 26

#include <stdio.h>
#include <check.h>
#include <zlib.h>
#include <fuse.h>
#include <stdlib.h>
#include <errno.h>

extern struct fuse_operations fs_ops;
extern void block_init(char *file);

struct path_list {
    char *path;
    int uid;
    int gid;
    int mode;
    size_t size;
    int ctime;
    int mtime;
};

struct path_list paths[] = {
    {"/", 0, 0, 040777, 4096, 1565283152, 1565283167},
    {"/file.1k", 500, 500, 0100666, 1000, 1565283152, 1565283152},
    {"/file.10", 500, 500, 0100666, 10, 1565283152, 1565283167},
    {"/dir-with-long-name", 0, 0, 040777, 4096, 1565283152, 1565283167},
    {"/dir-with-long-name/file.12k+", 0, 500, 0100666, 12289, 1565283152, 1565283167},
    {"/dir2", 500, 500, 040777, 8192, 1565283152, 1565283167},
    {"/dir2/twenty-seven-byte-file-name", 500, 500, 0100666, 1000, 1565283152, 1565283167},
    {"/dir2/file.4k+", 500, 500, 0100777, 4098, 1565283152, 1565283167},
    {"/dir3", 0, 500, 040777, 4096, 1565283152, 1565283167},
    {"/dir3/subdir", 0, 500, 040777, 4096, 1565283152, 1565283167},
    {"/dir3/subdir/file.4k-", 500, 500, 0100666, 4095, 1565283152, 1565283167},
    {"/dir3/subdir/file.8k-", 500, 500, 0100666, 8190, 1565283152, 1565283167},
    {"/dir3/subdir/file.12k", 500, 500, 0100666, 12288, 1565283152, 1565283167},
    {"/dir3/file.12k-", 0, 500, 0100777, 12287, 1565283152, 1565283167},
    {"/file.8k+", 500, 500, 0100666, 8195, 1565283152, 1565283167},
    {0}
};

START_TEST(getattr)
{
    struct stat sb;
    for (int i = 0; paths[i].path != NULL; i++) {
        printf("\n  - getattr %s", paths[i].path);
        int val = fs_ops.getattr(paths[i].path, &sb);
        ck_assert_int_eq(val, 0);
        ck_assert_int_eq(sb.st_uid, paths[i].uid);
        ck_assert_int_eq(sb.st_gid, paths[i].gid);
        ck_assert_int_eq(sb.st_mode, paths[i].mode);
        ck_assert_int_eq(sb.st_size, paths[i].size);
//        ck_assert_int_eq(sb.st_ctime, paths[i].ctime);
//        ck_assert_int_eq(sb.st_mtime, paths[i].mtime);
    }
    printf("\n");
}
END_TEST

struct dir_contents {
    char *path;
    char *contents[10];
};

struct dir_contents dirs[] = {
    {"/", {"dir2", "dir3", "dir-with-long-name", "file.10",
           "file.1k", "file.8k+", 0}},
    {"/dir-with-long-name", {"file.12k+", 0}},
    {"/dir2", {"file.4k+", "twenty-seven-byte-file-name", 0}},
    {"/dir3", {"file.12k-", "subdir", 0}},
    {"/dir3/subdir", {"file.12k", "file.4k-", "file.8k-", 0}},
    {0}
};

int test_2_filler(void *ptr, const char *name, const struct stat *stbuf,
           off_t off)
{
    struct dir_contents *dir = ptr;
    for (int i = 0; i < 10; i++)
        if (dir->contents[i] && strcmp(name, dir->contents[i]) == 0) {
            dir->contents[i] = NULL;
            return 0;
        }
    ck_abort_msg("readdir : result not found\n");
    return 0;
}

START_TEST(readdir)
{
    struct dir_contents *d = malloc(sizeof(dirs));
    memcpy(d, dirs, sizeof(dirs));
    for (int i = 0; d[i].path != NULL; i++) {
        printf("\n  - readdir %s", d[i].path);
        int rv = fs_ops.readdir(d[i].path, &d[i], test_2_filler, 0, NULL);
        ck_assert(rv >= 0);
        for (int j = 0; j < 10; j++)
            ck_assert_ptr_eq(d[i].contents[j], NULL);
    }
    free(d);
    printf("\n");
}
END_TEST

struct file_cksum {
    unsigned cksum;
    size_t   size;
    char    *path;
};
struct file_cksum cksums[] = {
    {1786485602, 1000, "/file.1k"},
    {855202508, 10, "/file.10"},
    {4101348955, 12289, "/dir-with-long-name/file.12k+"},
    {2575367502, 1000, "/dir2/twenty-seven-byte-file-name"},
    {799580753, 4098, "/dir2/file.4k+"},
    {4220582896, 4095, "/dir3/subdir/file.4k-"},
    {4090922556, 8190, "/dir3/subdir/file.8k-"},
    {3243963207, 12288, "/dir3/subdir/file.12k"},
    {2954788945, 12287, "/dir3/file.12k-"},
    {2112223143, 8195, "/file.8k+"},
    {0}
};

START_TEST(read_big)
{
    char seventeen[256];
    memset(seventeen, 17, 256);

    for (int i = 0; cksums[i].path != NULL; i++) {
        int buflen = cksums[i].size + 256;
        void *buf = malloc(buflen);
        memset(buf+cksums[i].size, 17, 256);
        printf("\n  - big_read %s", cksums[i].path);
        int len = fs_ops.read(cksums[i].path, buf, buflen, 0, NULL);
        ck_assert_int_ge(len, 0);
        unsigned crc = crc32(0, buf, len);

        ck_assert_int_eq(len, cksums[i].size);
        ck_assert_int_eq(crc, cksums[i].cksum);
        ck_assert_int_eq(memcmp(buf+cksums[i].size, seventeen, 256), 0);
        free(buf);
    }
    printf("\n");
}
END_TEST

int init_fs(void)
{
    block_init("test.img");
    fs_ops.init(NULL);
    return 0;
}

void read_n_at_a_time(char *path, void *buf, int chunk, int len)
{
    int n, offset = 0;
    while (1) {
        n = fs_ops.read(path, buf, chunk, offset, NULL);
        buf += n;
        offset += n;
        if (n == 0)
            break;
        if (n < chunk)
            ck_assert_int_eq(offset, len);
        else
            ck_assert_int_eq(n, chunk);
    }
}

#define N_ELEMENTS(x) sizeof((x)) / sizeof((x)[0])

START_TEST(read_little)
{
    int sizes[] = {17, 128, 511, 1020, 1024, 2050};
    char seventeen[256];
    memset(seventeen, 17, 256);

    for (int i = 0; i < N_ELEMENTS(sizes); i++) {
        int chunk = sizes[i];
        for (int j = 0; cksums[j].path != NULL; j++) {
            printf("\n  - little_read[%d] %s", chunk, cksums[j].path);
            int len = cksums[j].size;
            void *buf = malloc(len+256);
            memset(buf+len, 17, 256);

            read_n_at_a_time(cksums[j].path, buf, chunk, len);
            unsigned crc = crc32(0, buf, len);

            ck_assert_int_eq(crc, cksums[j].cksum);
            ck_assert_int_eq(memcmp(buf+cksums[j].size, seventeen, 256), 0);
            free(buf);
        }
    }
    printf("\n");
}
END_TEST

struct path_err {
    char *path;
    int err;
};
struct path_err bad1[] = {
    {"/not-a-file", ENOENT},
    {"/dir1/not-a-file", ENOENT},
    {"/not-a-dir/file.0", ENOENT},
    {"/file.8k+/file.0", ENOTDIR},
    {"/dir2/file.4k+/testing", ENOTDIR},
    {"/dir2/thisisaverylongnamelongerthan27chars", ENOENT},
    {0}
};

START_TEST(path_errs)
{
    struct stat sb;
    for (int i = 0; bad1[i].path != NULL; i++) {
        printf("\n  - path_err %s", bad1[i].path);
        int rv = fs_ops.getattr(bad1[i].path, &sb);
        ck_assert_int_eq(rv, -bad1[i].err);
    }
    printf("\n");
}
END_TEST

struct path_err bad_6[] = {
    {"/not-a-file", ENOENT},
    {"/dir2", EISDIR},
    {"/not-a-dir/file.12k+", ENOENT},
    {"/file.8k+/file.8k+", ENOTDIR},
    {0}
};

START_TEST(read_badpath)
{
    char buf[100];
    for (int i = 0; bad_6[i].path != NULL; i++) {
        printf("\n  - read_path_err %s", bad_6[i].path);
        int rv = fs_ops.read(bad_6[i].path, buf, sizeof(buf), 0, NULL);
        ck_assert_int_eq(rv, -bad_6[i].err);
    }
    printf("\n");
}
END_TEST

struct path_err bad_7[] = {
    {"/not-a-dir", ENOENT},
    {"/file.8k+", ENOTDIR},
    {"/not-a-dir/file.0", ENOENT},
    {"/dir2/file.4k+", ENOTDIR},
    {0}
};

int test_7_filler(void *ptr, const char *name, const struct stat *stbuf,
                  off_t off)
{
    ck_abort_msg("filler called on bad readdir");
    return 0;
}

START_TEST(readdir_badpath)
{
    for (int i = 0; bad_7[i].path != NULL; i++) {
        printf("\n  - readdir_path_err %s", bad_7[i].path);
        int rv = fs_ops.readdir(bad_7[i].path, NULL, test_7_filler, 0, NULL);
        ck_assert_int_eq(rv, -bad_7[i].err);
    }
    printf("\n");
}
END_TEST

START_TEST(test_statvfs)
{
    struct statvfs sv;
    int rv = fs_ops.statfs("/", &sv);
    ck_assert_int_eq(rv, 0);
    ck_assert_int_eq(sv.f_bsize, 4096);
    ck_assert_int_ge(sv.f_blocks, 398);
    ck_assert_int_le(sv.f_blocks, 400);
    ck_assert_int_eq(sv.f_bfree, 355);
    ck_assert_int_eq(sv.f_namemax, 27);
}
END_TEST


struct rename_case {
    char *src_path;
    char *dst_path;
    int ret;
};

struct rename_case err1[] = {
    {"/not-a-file", "/oooo", ENOENT},
    {"/dir1/file.tmp", "/oooo", ENOENT},
    {"/file.1k", "/file.1k", EEXIST},
    {"/dir2/file.4k+", "/dir3/somefile", EINVAL},
    {0}
};


START_TEST(test_bad_rename)
{
    for (int i = 0; err1[i].src_path != NULL; i++) {
        printf("\n  - rename_err %s => %s", err1[i].src_path, err1[i].dst_path);
        int rv = fs_ops.rename(err1[i].src_path, err1[i].dst_path);
        ck_assert_int_eq(rv, -err1[i].ret);
    }
    printf("\n");
}
END_TEST


struct rename_case renames[] = {
    {"/file.1k", "/file.tmp", 0},
    {"/file.tmp", "/file.1k", 0},
    {"/dir2/file.4k+", "/dir2/file.tmp", 0},
    {"/dir2/file.tmp", "/dir2/file.4k+", 0},
    {0}
};

START_TEST(test_rename)
{
    // 1: run renames
    for (int i = 0; renames[i].src_path != NULL; i++) {
        printf("\n  - rename %s => %s", renames[i].src_path, renames[i].dst_path);
        int rv = fs_ops.rename(renames[i].src_path, renames[i].dst_path);
        ck_assert_int_eq(rv, renames[i].ret);

        // src should not exists; dst exists
        struct stat sb;
        printf("\n  - getattr %s (should exist)", renames[i].dst_path);
        rv = fs_ops.getattr(renames[i].dst_path, &sb);
        ck_assert_int_eq(rv, 0);
        printf("\n  - getattr %s (should not exist)", renames[i].src_path);
        rv = fs_ops.getattr(renames[i].src_path, &sb);
        ck_assert_int_lt(rv, 0);
    }
    printf("\n");
}
END_TEST



struct chmod_case {
    char *path;
    mode_t mode;
    int ret;
};

struct chmod_case chmods[] = {
    {"/file.1k", 0777, ENOENT},
    {"/file.1k", 0666, ENOENT},
    {"/dir2/file.4k+", 0777, ENOENT},
    {"/dir2/file.4k+", 0666, ENOENT},
    {0}
};


START_TEST(test_chmod)
{
    for (int i = 0; chmods[i].path != NULL; i++) {
        struct stat sb;
        printf("\n  - prefetch mode of %s", chmods[i].path);
        int rv = fs_ops.getattr(chmods[i].path, &sb);
        ck_assert_int_eq(rv, 0);
        mode_t old_mode = sb.st_mode;


        printf("\n  - chmod %s %o", chmods[i].path, chmods[i].mode);
        rv = fs_ops.chmod(chmods[i].path, chmods[i].mode);
        ck_assert_int_eq(rv, 0);

        // check mode
        printf("\n  - check mode of %s (should be %o)", chmods[i].path, chmods[i].mode);
        rv = fs_ops.getattr(chmods[i].path, &sb);
        ck_assert_int_eq(rv, 0);
        ck_assert_int_eq(sb.st_mode & 0777, chmods[i].mode & 0777);  // permission should change
        ck_assert_int_eq(sb.st_mode & S_IFMT, old_mode & S_IFMT);  // others should not
    }
    printf("\n");
}
END_TEST


struct chmod_case err2[] = {
    {"/not-a-file", 0777, ENOENT},
    {"/not-a-dir/not-a-file", 0777, ENOENT},
    {0}
};


START_TEST(test_bad_chmod)
{
    for (int i = 0; err2[i].path != NULL; i++) {
        printf("\n  - chmod_err %s %o", err2[i].path, err2[i].mode);
        int rv = fs_ops.chmod(err2[i].path, err2[i].mode);
        ck_assert_int_eq(rv, -1*err2[i].ret);
    }
    printf("\n");
}
END_TEST



int main(int argc, char **argv)
{
    init_fs();

    Suite *s = suite_create("fs5600");
    TCase *tc = tcase_create("read_mostly");

    tcase_add_test(tc, test_statvfs);
    tcase_add_test(tc, getattr);
    tcase_add_test(tc, path_errs);
    tcase_add_test(tc, readdir_badpath);
    tcase_add_test(tc, readdir);
    tcase_add_test(tc, read_badpath);
    tcase_add_test(tc, read_little);
    tcase_add_test(tc, read_big);
    tcase_add_test(tc, test_rename);
    tcase_add_test(tc, test_bad_rename);
    tcase_add_test(tc, test_bad_chmod);
    tcase_add_test(tc, test_chmod);

    suite_add_tcase(s, tc);
    SRunner *sr = srunner_create(s);
//    srunner_set_fork_status(sr, CK_NOFORK);

    srunner_run_all(sr, CK_VERBOSE);
    int number_failed = srunner_ntests_failed(sr);
    srunner_free(sr);
    return (number_failed == 0) ? EXIT_SUCCESS : EXIT_FAILURE;
}

struct fuse_context *fuse_get_context(void)
{
    return NULL;
}
