#![feature(ptr_internals)]

use std::alloc::{alloc, realloc, Layout};
use std::mem;
use std::process;
use std::ptr::Unique;

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
}
