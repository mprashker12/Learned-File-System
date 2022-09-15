#
# file:        Makefile - NEU file system
#

CFLAGS = -ggdb3 -Wall -O0
LDLIBS = -lcheck -lz -lm -lsubunit -lrt -lpthread -lfuse

all: lab1fuse test.img test2.img test1 test2

test1: test1.o fs5600.o misc.o
	$(CC) $^ $(LDLIBS) -o $@

test2: test2.o fs5600.o misc.o
	$(CC) $^ $(LDLIBS) -o $@

lab1fuse: misc.o fs5600.o lab1fuse.o
	$(CC) $^ $(LDLIBS) -o $@

testa: all
	./test1

testb: all
	./test2

# force test.img, test2.img to be rebuilt each time
.PHONY: test/test.img test/test2.img

test.img: 
	python2 test/gen-disk.py -q test/disk1.in test/test.img

test2.img: 
	python2 test/gen-disk.py -q testing/disk2.in test/test2.img

clean: 
	rm -f *.o lab1fuse test/test.img test/test2.img test1 test2 diskfmt.pyc
