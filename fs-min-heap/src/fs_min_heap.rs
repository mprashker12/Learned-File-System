use std::rc::Rc;
use std::cell::RefCell;

pub trait block_reader {
    type Item : Copy 
                + Clone 
                + Default
                + std::cmp::Eq 
                + std::cmp::Ord 
                + std::cmp::PartialEq 
                + std::cmp::PartialOrd;
    
    
    fn new() -> Self;

    fn capacity(&self) -> usize;
    fn block_size(&self) -> usize;
    
    fn swap(&mut self, i: &usize, j: &usize);
    
    fn read(&self, index: &usize) -> Option<&Self::Item>;
    fn write(&mut self, index: &usize, val : Self::Item);
    
    // For a given index, what block contains it?
    fn block_containing_index(&self, index: &usize) -> Option<usize>;
}


pub struct FsMinHeap<BR: block_reader> {
    pub size: usize,
    pub capacity: usize,
    pub disk: Rc<RefCell<BR>>,
}


/// Implementation of a Min-Heap on top of an abstract Disk
impl<BR: block_reader> FsMinHeap<BR> {
    pub fn new() -> Self {
        let disk = BR::new();
        Self { size: 0, capacity: disk.capacity(), disk: Rc::new(RefCell::new(disk))}
    }

    pub fn insert(&mut self, elem: BR::Item) {
        if self.size == self.capacity {
            panic!("Trying to insert on full heap")
        }
        self.size += 1;
        self.disk.borrow_mut().write(&self.size, elem);
        self.sift_up(); 
    }

    pub fn pop(&mut self) -> Option<BR::Item> {
        if self.size == 0 {return None;}

        let root_index = self.root_index();
        let last_index = self.size;
        
        let ret = *self.disk.clone().borrow().read(&root_index).unwrap();

        self.disk.borrow_mut().swap(&root_index, &last_index);
        self.size -= 1;
        self.sift_down();
        
        Some(ret)
    }

    fn sift_down(&mut self) {
        let mut curr_index = self.root_index();
        loop {
            let left_child = self.left_child(curr_index);
            let right_child = self.right_child(curr_index);

            match (left_child.is_some(), right_child.is_some()) {
                (true, true) => {
                    let left_child = left_child.unwrap();
                    let right_child = right_child.unwrap();

                    let left_smaller = self.disk.borrow().read(&left_child).unwrap() <= self.disk.borrow().read(&right_child).unwrap();

                    if left_smaller {
                        if self.disk.borrow().read(&left_child).unwrap() < self.disk.borrow().read(&curr_index).unwrap() {
                            self.swap(&left_child, &curr_index);
                            curr_index = left_child;
                        } else {    
                            break;
                        }
                    } else {
                        if self.disk.borrow().read(&right_child).unwrap() < self.disk.borrow().read(&curr_index).unwrap() {
                            self.swap(&right_child, &curr_index);
                            curr_index = right_child;
                        } else {    
                            break;
                        }
                    }
                }
                (true, false) => {
                    let left_child = left_child.unwrap();
                    if self.disk.borrow().read(&curr_index).unwrap() <= self.disk.borrow().read(&left_child).unwrap() {
                        break;
                    }
                    self.swap(&curr_index, &left_child);
                    curr_index = left_child;
                }
                (false, true) => {
                    let right_child = right_child.unwrap();
                    if self.disk.borrow().read(&curr_index).unwrap() <= self.disk.borrow().read(&right_child).unwrap() {
                        break;
                    }
                    self.swap(&curr_index, &right_child);
                    curr_index = right_child;
                }
                (false, false) => {break;}
            }
        }
    }

    fn sift_up(&mut self) {
        let mut curr_index = self.size;
        loop {
            let parent = self.parent(&curr_index);
            if parent.is_none() {break;}
            
            let parent = parent.unwrap();
            if self.disk.borrow().read(&parent).unwrap() <= self.disk.borrow().read(&curr_index).unwrap() {break;}
            
            self.swap(&curr_index, &parent);
            curr_index = parent;
        }
    }
  

    fn swap(&mut self, i: &usize, j: &usize) {
        if *i == 0 || *i > self.size || *j == 0 || *j > self.size {
            panic!("Trying to swap {} {} but size is {}", i, j, self.size);
        }
        
        let val_i = *self.disk.clone().borrow().read(i).unwrap();
        let val_j = *self.disk.clone().borrow().read(j).unwrap();
        self.disk.clone().borrow_mut().write(i, val_j);
        self.disk.clone().borrow_mut().write(j, val_i);
    }

    fn root_index(&self) -> usize {1}

    fn left_child(&self, index: usize) -> Option<usize> {
        let left_child = 2*index;
        if left_child <= self.size {return Some(left_child);}
        None
    }

    fn right_child(&self, index: usize) -> Option<usize> {
        let right_child = 2*index + 1;
        if right_child <= self.size {return Some(right_child);}
        None
    }

    fn parent(&self, index: &usize) -> Option<usize> {
        if *index == 1 {return None;}
        Some(index/2)
    }
}