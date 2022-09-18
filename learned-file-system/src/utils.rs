use std::ops::{Add, Div, Sub};

pub mod block_file;
pub mod bitmask;

pub fn div_ceil<T : Add<Output=T> + Sub<Output=T> + Div<Output=T> + Copy + From<u8>>(n: T, d: T) -> T {
    (n + d - (T::from(1u8)))/d
}
