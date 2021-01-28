#![feature(ptr_internals)]

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::mem;
use std::ops::{Deref, DerefMut};
use std::process;
use std::ptr::{self, Unique};

pub struct Vec<T> {
    ptr: Unique<T>,
    cap: usize,
    len: usize,
}

impl<T> Vec<T> {
    pub fn new() -> Self {
        assert_ne!(mem::size_of::<T>(), 0, "I'm not ready to handle ZSTs ):");

        Vec {
            ptr: Unique::dangling(),
            len: 0,
            cap: 0,
        }
    }

    fn grow(&mut self) {
        unsafe {
            let elem_size = mem::size_of::<T>();
            let align = mem::align_of::<T>();

            let (new_cap, ptr) = if self.cap == 0 {
                let layout = Layout::from_size_align_unchecked(elem_size, align);
                let ptr = alloc(layout);

                (1, ptr)
            } else {
                let new_cap = self.cap * 2;
                let old_num_bytes = self.cap * elem_size;

                let layout = Layout::from_size_align_unchecked(old_num_bytes, align);

                assert!(
                    old_num_bytes <= (isize::MAX as usize) / 2,
                    "The capacity has overflown!"
                );

                let new_num_bytes = old_num_bytes * 2;

                let ptr = realloc(self.ptr.as_ptr() as *mut _, layout, new_num_bytes);

                (new_cap, ptr)
            };

            if ptr.is_null() {
                process::abort();
            }

            self.ptr = Unique::new_unchecked(ptr as *mut _);
            self.cap = new_cap;
        }
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap {
            self.grow();
        }

        unsafe {
            ptr::write(self.ptr.as_ptr().offset(self.len as isize), elem);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;

            unsafe { Some(ptr::read(self.ptr.as_ptr().offset(self.len as isize))) }
        }
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        if self.cap != 0 {
            while let Some(_) = self.pop() {}

            let elem_size = mem::size_of::<T>();
            let align = mem::align_of::<T>();
            let num_bytes = elem_size * self.cap;

            unsafe {
                let layout = Layout::from_size_align_unchecked(num_bytes, align);

                dealloc(self.ptr.as_ptr() as *mut _, layout);
            }
        }
    }
}
