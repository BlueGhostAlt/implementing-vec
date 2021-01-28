#![feature(ptr_internals)]

use std::ptr::Unique;

pub struct Vec<T> {
    ptr: Unique<T>,
    cap: usize,
    len: usize,
}
