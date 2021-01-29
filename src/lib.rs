#![feature(ptr_internals)]

use std::alloc::{alloc, dealloc, realloc, Layout};
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::process;
use std::ptr::{self, Unique};

struct RawVec<T> {
    ptr: Unique<T>,
    cap: usize,
}

impl<T> RawVec<T> {
    fn new() -> Self {
        let cap = if mem::size_of::<T>() == 0 {
            usize::MAX
        } else {
            0
        };

        RawVec {
            ptr: Unique::dangling(),
            cap,
        }
    }

    fn grow(&mut self) {
        unsafe {
            let elem_size = mem::size_of::<T>();

            assert_ne!(elem_size, 0, "The capacity has overflown!");

            let align = mem::align_of::<T>();

            let (new_cap, ptr) = if self.cap == 0 {
                let layout = Layout::from_size_align_unchecked(elem_size, align);
                let ptr = alloc(layout);

                (1, ptr)
            } else {
                let new_cap = self.cap * 2;

                let layout = Layout::from_size_align_unchecked(self.cap * elem_size, align);
                let ptr = realloc(self.ptr.as_ptr() as *mut _, layout, new_cap * elem_size);

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

impl<T> Drop for RawVec<T> {
    fn drop(&mut self) {
        let elem_size = mem::size_of::<T>();

        if self.cap != 0 && elem_size != 0 {
            let align = mem::align_of::<T>();
            let num_bytes = elem_size * self.cap;

            unsafe {
                let layout = Layout::from_size_align_unchecked(num_bytes, align);
                dealloc(self.ptr.as_ptr() as *mut _, layout);
            }
        }
    }
}

pub struct Vec<T> {
    buf: RawVec<T>,
    len: usize,
}

impl<T> Vec<T> {
    fn ptr(&self) -> *mut T {
        self.buf.ptr.as_ptr()
    }

    fn cap(&self) -> usize {
        self.buf.cap
    }

    pub fn new() -> Self {
        Vec {
            buf: RawVec::new(),
            len: 0,
        }
    }

    pub fn push(&mut self, elem: T) {
        if self.len == self.cap() {
            self.buf.grow();
        }

        unsafe {
            ptr::write(self.ptr().add(self.len), elem);
        }

        self.len += 1;
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            None
        } else {
            self.len -= 1;

            unsafe {
                let elem = ptr::read(self.ptr().add(self.len));
                Some(elem)
            }
        }
    }

    pub fn insert(&mut self, index: usize, elem: T) {
        assert!(index <= self.len, "Insertion index is out of bounds!");

        if self.len == self.cap() {
            self.buf.grow();
        }

        unsafe {
            if index < self.len {
                ptr::copy(
                    self.ptr().add(index),
                    self.ptr().add(index + 1),
                    self.len - index,
                );
            }
            ptr::write(self.ptr().add(index), elem);

            self.len += 1;
        }
    }

    pub fn remove(&mut self, index: usize) -> T {
        assert!(index < self.len, "Removal index is out of bounds!");

        unsafe {
            self.len -= 1;

            let elem = ptr::read(self.ptr().add(index));
            ptr::copy(
                self.ptr().add(index + 1),
                self.ptr().add(index),
                self.len - index,
            );

            elem
        }
    }

    pub fn into_iter(self) -> IntoIter<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            let buf = ptr::read(&self.buf);
            mem::forget(self);

            IntoIter { iter, _buf: buf }
        }
    }

    pub fn drain(&mut self) -> Drain<T> {
        unsafe {
            let iter = RawValIter::new(&self);

            self.len = 0;

            Drain {
                iter,
                vec: PhantomData,
            }
        }
    }
}

impl<T> Default for Vec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for Vec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr(), self.len) }
    }
}

impl<T> DerefMut for Vec<T> {
    fn deref_mut(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr(), self.len) }
    }
}

impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        while self.pop().is_some() {}
    }
}

struct RawValIter<T> {
    start: *const T,
    end: *const T,
}

impl<T> RawValIter<T> {
    unsafe fn new(slice: &[T]) -> Self {
        RawValIter {
            start: slice.as_ptr(),
            end: if mem::size_of::<T>() == 0 {
                ((slice.as_ptr() as usize) + slice.len()) as *const _
            } else if slice.is_empty() {
                slice.as_ptr()
            } else {
                slice.as_ptr().add(slice.len())
            },
        }
    }
}

impl<T> Iterator for RawValIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                let result = ptr::read(self.start);
                self.start = if mem::size_of::<T>() == 0 {
                    (self.start as usize + 1) as *const _
                } else {
                    self.start.offset(1)
                };

                Some(result)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let elem_size = mem::size_of::<T>();
        let elem_size = if elem_size == 0 { 1 } else { elem_size };
        let size = self.end as usize - self.start as usize;

        let len = size / elem_size;

        (len, Some(len))
    }
}

impl<T> DoubleEndedIterator for RawValIter<T> {
    fn next_back(&mut self) -> Option<T> {
        if self.start == self.end {
            None
        } else {
            unsafe {
                self.end = if mem::size_of::<T>() == 0 {
                    (self.end as usize - 1) as *const _
                } else {
                    self.end.offset(-1)
                };
                let elem = ptr::read(self.end);

                Some(elem)
            }
        }
    }
}

pub struct IntoIter<T> {
    _buf: RawVec<T>,
    iter: RawValIter<T>,
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for _ in &mut self.iter {}
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}

pub struct Drain<'a, T: 'a> {
    vec: PhantomData<&'a mut Vec<T>>,
    iter: RawValIter<T>,
}

impl<'a, T> Drop for Drain<'a, T> {
    fn drop(&mut self) {
        for _ in &mut self.iter {}
    }
}

impl<'a, T> Iterator for Drain<'a, T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.iter.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for Drain<'a, T> {
    fn next_back(&mut self) -> Option<T> {
        self.iter.next_back()
    }
}
