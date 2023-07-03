use std::{ptr::{NonNull, copy_nonoverlapping}, alloc::{Layout}, ops::{Index, IndexMut}};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ListError {
    #[error("Could not allocate memory for list")]
    Allocation,
}

type Error = ListError;
type Result<T> = std::result::Result<T, Error>;

pub struct List<T> {
    ptr: NonNull<T>,
    len: usize,
    cap: usize,
}

impl<T> List<T> {
    pub fn new() -> Self {
        Self {
            ptr: NonNull::dangling(),
            len: 0,
            cap: 0,
        }
    }

    pub fn with_capacity(cap: usize) -> Result<Self> {
        let ptr = Self::alloc(cap)?;
        Ok(Self {
            ptr,
            len: 0,
            cap: cap,
        })
    }

    pub fn add(&mut self, value: T) -> Result<()> {
        self.ensure_cap(1)?;

        let ptr = self.ptr.as_ptr();
        unsafe {
            *ptr.add(self.len) = value;
        }
        self.len += 1;
        Ok(())
    }

    pub fn remove_last(&mut self) -> Option<T> {
        match self.len {
            0 => None,
            _ => {
                self.len -= 1;
                let ptr = self.ptr.as_ptr();
                unsafe {
                    let ptr = ptr.add(self.len);
                    Some(std::ptr::read(ptr))
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn cap(&self) -> usize {
        self.cap
    }

    fn ensure_cap(&mut self, additional: usize) -> Result<()> {
        if self.len + additional <= self.cap {
            return Ok(());
        }

        let new_cap = (self.cap + 1).next_power_of_two().max(4);
        let new_ptr = Self::alloc(new_cap)?;

        unsafe {
            let curr_ptr = self.ptr.as_ptr();
            copy_nonoverlapping(curr_ptr, new_ptr.as_ptr(), self.len);
        };

        self.ptr = new_ptr;
        self.cap = new_cap;

        Ok(())
    }

    fn alloc(cap: usize) -> Result<NonNull<T>> {
        let layout = Layout::array::<T>(cap).map_err(|_| Error::Allocation)?;
        let allocation = unsafe { std::alloc::alloc(layout) as *mut T };
        NonNull::new(allocation).ok_or(Error::Allocation)
    }
}

impl<T> Index<usize> for List<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.len {
            panic!("Index out of bounds");
        }

        let ptr = self.ptr.as_ptr();
        unsafe { ptr.add(index).as_ref().unwrap() }
    }
}

impl<T> IndexMut<usize> for List<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.len {
            panic!("Index out of bounds");
        }

        let ptr = self.ptr.as_ptr();
        unsafe { ptr.add(index).as_mut().unwrap() }
    }
}

mod tests {
    use super::*;

    #[test]
    fn new() {
        let list = List::<usize>::new();
        assert_eq!(0, list.len);
        assert_eq!(0, list.cap);
    }

    #[test]
    fn capacity() {
        let list = List::<usize>::with_capacity(8).unwrap();
        assert_eq!(0, list.len);
        assert_eq!(8, list.cap);
    }

    #[test]
    fn add() {
        let mut list = List::<usize>::new();
        list.add(1).unwrap();
        assert_eq!(1, list.len);
        assert_eq!(4, list.cap);
        assert_eq!(1, *&list[0]);
        assert_eq!(1, list[0]);
    }

    #[test]
    fn remove_last() {
        let mut list = List::<usize>::new();
        list.add(1).unwrap();
        let last = list.remove_last().unwrap();
        assert_eq!(1, last);
        assert_eq!(0, list.len);
        assert_eq!(4, list.cap);
    }

    #[test]
    fn mutate_in_place() {
        let mut list = List::<usize>::new();
        list.add(1).unwrap();

        let value = &mut list[0];
        *value = 2;

        assert_eq!(1, list.len);
        assert_eq!(4, list.cap);
        assert_eq!(2, *&list[0]);
        assert_eq!(2, list[0]);
    }
}
