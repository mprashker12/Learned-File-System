pub mod fs_min_heap;
use fs_min_heap::{block_reader, FsMinHeap};


pub struct VecDisk {
    data: Vec<usize>,
}

impl block_reader for VecDisk {
    
    type Item = usize;
    
    fn new() -> Self {
        VecDisk { data: vec![0; (1 << 20) + 1] }
    }

    fn capacity(&self) -> usize {
        self.data.len() - 1
    }

    fn block_size(&self) -> usize {
        1 << 12
    }

    fn swap(&mut self, i: &usize, j: &usize) {
        self.data.swap(*i, *j);
    }


    fn read(&self, index: &usize) -> Option<&Self::Item> {
        println!("Reading {}", index);
        self.data.get(*index)
    }

    fn write(&mut self, index: &usize, val : Self::Item) {
        println!("Writing {}", index);
        self.data[*index] = val;
    }
    
    fn block_containing_index(&self, index: &usize) -> Option<usize> {
        if *index > self.capacity() {
            return None;
        }
        Some(*index/self.block_size())
    }

}

#[test]
pub fn basic_heap() {
    let mut min_heap = FsMinHeap::<VecDisk>::new();
    min_heap.insert(100);
    min_heap.insert(50);
    let x = min_heap.pop().unwrap();
    let y = min_heap.pop().unwrap();
    println!("{}", x);
    println!("{}", y);
}

fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        println!("Useage: [filename]");
        std::process::exit(1);
    }

    let file_name = args.get(1).unwrap();
    Ok(())
}
