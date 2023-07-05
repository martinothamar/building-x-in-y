use std::{mem::{MaybeUninit}, fmt::{Debug, Formatter, self}, ptr::addr_of_mut};
use thiserror::Error;

use crate::internal::*;

#[derive(Error, Debug)]
pub enum RingBufferError {
    #[error("The ring buffer is empty")]
    Empty,
    #[error("The ring buffer is full")]
    Full,
}

type Error = RingBufferError;
type Result<T> = std::result::Result<T, Error>;

pub struct RingBuffer<T, const N: usize> {
    inner: Box<RingBufferData<T, N>>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        let mut data = Box::<RingBufferData<T, N>>::new_uninit();
        unsafe { RingBufferData::init(data.as_mut_ptr()) };
        let data = unsafe { data.assume_init() };
        Self {
            inner: data,
        }
    }

    pub fn push(&mut self) -> Result<&mut T> {
        if self.is_full() {
            return Err(Error::Full);
        }

        let result = unsafe { self.inner.storage[Self::mask(self.inner.write_index)].assume_init_mut() };
        self.inner.write_index = Self::mask2(self.inner.write_index + 1);

        Ok(result)
    }

    pub fn pop(&mut self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        let value: &T = unsafe { self.inner.storage[Self::mask(self.inner.read_index)].assume_init_ref() };
        self.inner.read_index = Self::mask2(self.inner.read_index + 1);

        Some(value)
    }

    pub fn len(&self) -> usize {
        let wrap_offset = 2 * N * (self.inner.write_index < self.inner.read_index) as usize;
        let adjusted_write_index = self.inner.write_index + wrap_offset;
        return adjusted_write_index - self.inner.read_index;
    }

    pub fn is_empty(&self) -> bool {
        self.inner.read_index == self.inner.write_index
    }

    pub fn is_full(&self) -> bool {
        Self::mask2(self.inner.write_index + N) == self.inner.read_index
    }

    fn mask(index: usize) -> usize {
        return index & (N - 1);
    }

    fn mask2(index: usize) -> usize {
        return index & ((2 * N) - 1);
    }
}


impl<T: Debug, const N: usize> Debug for RingBuffer<T, N>{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let storage: &dyn Debug = unsafe { &self.inner.storage.as_ptr().cast::<[T; N]>().read() };
        f.debug_struct("RingBuffer")
            .field("storage", storage)
            .field("read_index", &self.inner.read_index)
            .field("write_index", &self.inner.write_index)
            .finish()
    }
}

struct RingBufferData<T, const N: usize> {
    storage: [MaybeUninit<T>; N],
    read_index: usize,
    write_index: usize,
}
impl<T, const N: usize> RingBufferData<T, N> {
    #[inline(never)]
    unsafe fn init(slot: *mut RingBufferData<T, N>) {
        addr_of_mut!((*slot).storage).write(MaybeUninit::uninit().assume_init());
        addr_of_mut!((*slot).read_index).write(0);
        addr_of_mut!((*slot).write_index).write(0);
    }
}
