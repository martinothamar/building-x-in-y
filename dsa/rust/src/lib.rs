use std::{mem::MaybeUninit, fmt::{Debug, Formatter, self}};

pub struct RingBuffer<T, const N: usize> {
    storage: [MaybeUninit<T>; N],
    read_index: usize,
    write_index: usize,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            storage: unsafe { MaybeUninit::uninit().assume_init() },
            read_index: 0,
            write_index: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.read_index == self.write_index
    }
}

impl<T: Debug, const N: usize> Debug for RingBuffer<T, N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let storage: &dyn Debug = unsafe { &self.storage.as_ptr().cast::<[T; N]>().read() };
        f.debug_struct("RingBuffer")
            .field("storage", storage)
            .field("read_index", &self.read_index)
            .field("write_index", &self.write_index)
            .finish()
    }
}
