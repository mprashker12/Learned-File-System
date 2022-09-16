use fuse::mount;
use std::env;
use std::process::exit;
use learned_file_system::LearnedFileSystem;

use std::fs::File;

use learned_file_system::utils::block_file::BlockFileWrapper;


const BLOCK_SIZE: usize = 4096;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() != 4 {
        println!("usage: ./lab1fuse -image disk.img directory");
        println!("             disk.img  - name of the image file to mount");
        println!("             directory - directory to mount it on");
        exit(1);
    }

    let image_name = args.get(2).unwrap();
    let image = File::open(image_name).unwrap();
    let num_blocks = 1000; //TODO
    let block_device = BlockFileWrapper::new(BLOCK_SIZE, num_blocks, image);

    let mountpoint = args.get(3).unwrap();

    let l = LearnedFileSystem::new(block_device);
    mount(l, mountpoint, &[]).unwrap();
}