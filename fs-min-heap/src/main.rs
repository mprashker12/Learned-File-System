#![feature(file_create_new)]

pub mod fs_min_heap;
use fs_min_heap::{block_reader, FsMinHeap};

use std::fmt::write;
use std::ops::Add;
use fs::File;
use std::fs;
use std::io::{BufWriter, Write};
use std::cell::RefCell;
use rand::Rng;



pub struct VecDisk {
    pub disk_accesses: RefCell<Vec<usize>>,
    pub data: Vec<usize>,
}

impl block_reader for VecDisk {
    
    type Item = usize;
    
    fn new() -> Self {
        VecDisk { disk_accesses: RefCell::new(vec![]), data: vec![0; (1 << 20) + 1] }
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
        //LOGGING
        self.disk_accesses.borrow_mut().push(*index);
        
        self.data.get(*index)
    }

    fn write(&mut self, index: &usize, val : Self::Item) {
        //LOGGING
        self.disk_accesses.borrow_mut().push(*index);
        
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
    min_heap.insert(75);
    min_heap.insert(42);
    let x = min_heap.pop().unwrap();
    let y = min_heap.pop().unwrap();
    assert_eq!(x, 42);
    assert_eq!(y, 50);
}

fn write_vec(file_path: &str, data: &Vec<usize>) {
    let mut write_file = fs::File::create_new(file_path).unwrap();
    for i in data.iter() {
        write_file.write(i.to_string().as_bytes());
        write_file.write("\n".as_bytes());
    }
}

fn run_experiment_round(round_number: usize) {
    let mut min_heap = FsMinHeap::<VecDisk>::new();
    for _ in 0..1000 {
        let num: usize = rand::thread_rng().gen_range(0..100);
        min_heap.insert(num);
    }
    for _ in 0..1000 {
        min_heap.pop();
        let num = rand::thread_rng().gen_range(0..100);
        min_heap.insert(num);
    }

    let access_patterns = min_heap.disk.borrow().disk_accesses.borrow().clone();

    let mut path = "exp_results/".to_string();
    path.push_str(&round_number.to_string());
    path.push_str(".txt");

    write_vec(path.as_str(), &access_patterns);
}

fn main()  {

    for round_number in 1..100 {
        run_experiment_round(round_number);
    }
}
