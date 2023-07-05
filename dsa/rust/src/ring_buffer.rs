use std::{
    fmt::{self, Debug, Formatter},
    mem::{self, MaybeUninit},
    ptr::addr_of_mut,
};
use thiserror::Error;

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
    storage: [MaybeUninit<T>; N],
    read_index: usize,
    write_index: usize,
}

impl<T, const N: usize> RingBuffer<T, N>
where
    [(); mem::size_of::<T>()]:,
{
    const ASSERTS: () = {
        assert!(
            is_power_of_two::<{ mem::size_of::<T>() }>(),
            "\nSize of T must be a power of two"
        );
        assert!(
            is_power_of_two::<N>(),
            "\nNumber of elements (N) must be a power of two"
        );
    };

    /// Allocates a new `RingBuffer<T, N>` on the heap
    ///
    /// ```
    /// use dsa::ring_buffer::RingBuffer;
    ///
    /// let mut rb = RingBuffer::<usize, 256>::new_heap();
    ///
    /// println!("{rb:?}");
    /// ```
    // #[inline(never)] Uncomment inlining to more easily inspect generated assembly (see Makefile)
    pub fn new_heap() -> Box<Self> {
        _ = Self::ASSERTS;

        // This is unfortunately the only sane syntax
        // I could come up with for initializing directly into a heap allocation
        // The normal `Box::new` syntax may end up doing memcpy's
        // which can overflow the stack if N is large
        let mut data: Box<MaybeUninit<RingBuffer<T, N>>> = Box::new_uninit();
        unsafe {
            let slot = data.as_mut_ptr();
            addr_of_mut!((*slot).read_index).write(0);
            addr_of_mut!((*slot).write_index).write(0);
            data.assume_init()
        }
    }

    #[allow(dead_code)]
    fn new_heap_may_overflow() -> Box<Self> {
        _ = Self::ASSERTS;

        // Do _NOT_ use, this is simply here to demonstrate that
        // for large N we can overflow the stack, even if the
        // intention is to allocate on the heap.
        // This is why the `new_heap` function exists which creates
        // an unitialized box and only initializes the indices
        Box::new(Self {
            storage: unsafe { MaybeUninit::uninit().assume_init() },
            read_index: 0,
            write_index: 0,
        })
    }

    /// Allocates a new `RingBuffer<T, N>` on the stack (inline)
    ///
    /// ```
    /// use dsa::ring_buffer::RingBuffer;
    ///
    /// let mut rb = RingBuffer::<usize, 256>::new_inline();
    ///
    /// println!("{rb:?}");
    /// ```
    // #[inline(never)]
    pub fn new_inline() -> Self {
        _ = Self::ASSERTS;

        Self {
            storage: unsafe { MaybeUninit::uninit().assume_init() },
            read_index: 0,
            write_index: 0,
        }
    }

    /// Grabs the next available value for writing, returning a mutable reference for you to write to
    /// Can return either of
    /// * `Result::Full`
    /// * `Result::Ok(&mut T)`
    ///
    /// ```
    /// use dsa::ring_buffer::RingBuffer;
    /// use std::result::Result;
    /// use std::error::Error;
    ///
    /// fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut rb = RingBuffer::<usize, 256>::new_inline();
    ///
    ///     *rb.push()? = 1337;
    ///     Ok(())
    /// }
    /// ```
    pub fn push(&mut self) -> Result<&mut T> {
        if self.is_full() {
            return Err(Error::Full);
        }

        let result = unsafe { self.storage[Self::mask(self.write_index)].assume_init_mut() };
        self.write_index = Self::mask2(self.write_index + 1);

        Ok(result)
    }

    /// Grabs the next available value for reading, returning an optional shared reference to T
    /// Can return either of
    /// * `Result::Empty`
    /// * `Result::Ok(&T)`
    ///
    /// ```
    /// use dsa::ring_buffer::RingBuffer;
    /// use std::result::Result;
    /// use std::error::Error;
    ///
    /// fn main() -> Result<(), Box<dyn Error>> {
    ///     let mut rb = RingBuffer::<usize, 256>::new_inline();
    ///
    ///     *rb.push()? = 1337;
    ///     let value: Option<&usize> = rb.pop();
    ///     assert!(value.is_some());
    ///     Ok(())
    /// }
    /// ```
    pub fn pop(&mut self) -> Option<&T> {
        if self.is_empty() {
            return None;
        }

        let value: &T = unsafe { self.storage[Self::mask(self.read_index)].assume_init_ref() };
        self.read_index = Self::mask2(self.read_index + 1);

        Some(value)
    }

    /// Returns the length of the `RingBuffer<T, N>`
    pub fn len(&self) -> usize {
        let wrap_offset = 2 * N * (self.write_index < self.read_index) as usize;
        let adjusted_write_index = self.write_index + wrap_offset;
        return adjusted_write_index - self.read_index;
    }

    /// Returns a boolean value indicating wether or not the `RingBuffer<T, N>` is empty or not
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.read_index == self.write_index
    }

    /// Returns a boolean value indicating wether or not the `RingBuffer<T, N>` iss full or not
    #[inline(always)]
    pub fn is_full(&self) -> bool {
        Self::mask2(self.write_index + N) == self.read_index
    }

    #[inline(always)]
    fn mask(index: usize) -> usize {
        return index & (N - 1);
    }

    #[inline(always)]
    fn mask2(index: usize) -> usize {
        return index & ((2 * N) - 1);
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

const fn is_power_of_two<const N: usize>() -> bool {
    N != 0 && N & (N - 1) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heap_construction() {
        let rb = RingBuffer::<usize, 256>::new_heap();
        assert_eq!(0, rb.len());
    }

    #[test]
    fn inline_construction() {
        let rb = RingBuffer::<usize, 256>::new_inline();
        assert_eq!(0, rb.len());
    }

    #[test]
    fn push_simple() {
        let mut rb = RingBuffer::<usize, 256>::new_inline();
        assert_eq!(0, rb.len());
        *rb.push().unwrap() = 1337;
        assert_eq!(1, rb.len());
    }

    #[test]
    fn push_pop() {
        let mut rb = RingBuffer::<usize, 256>::new_inline();
        assert_eq!(0, rb.len());
        *rb.push().unwrap() = 1337;
        assert_eq!(1, rb.len());
        _ = rb.pop().unwrap();
        assert_eq!(0, rb.len());
    }

    #[test]
    fn err_on_empty() {
        let mut rb = RingBuffer::<usize, 8>::new_inline();
        assert!(rb.pop().is_none());
    }

    #[test]
    fn err_on_full() {
        let mut rb = RingBuffer::<usize, 8>::new_inline();
        for i in 1..9 {
            *rb.push().unwrap() = i;
        }

        assert!(rb.push().is_err());
    }

    #[test]
    fn wrap_around() {
        let mut rb = RingBuffer::<usize, 8>::new_inline();

        for _ in 0..4 {
            for i in 1..5 {
                assert_eq!(i - 1, rb.len());
                *rb.push().unwrap() = i;
                assert_eq!(i, rb.len());
            }

            for i in 1..5 {
                let value = rb.pop().unwrap();
                assert_eq!(i, *value);
            }
        }
    }

    // NOTE: the commented tests below are there to verify

    // #[test]
    // #[should_panic]
    // fn large_ringbuffer_inline() {
    //     const N: usize = 1024 * 1024;
    //     let mut rb = RingBuffer::<usize, N>::new_inline();
    //     *rb.push().unwrap() = 1;
    // }

    // #[test]
    // #[should_panic]
    // fn large_ringbuffer_heap_dangerous() {
    //     const N: usize = 1024 * 1024;
    //     let mut rb = RingBuffer::<usize, N>::new_heap_may_overflow();
    //     *rb.push().unwrap() = 1;
    // }

    #[test]
    fn large_ringbuffer_heap() {
        const N: usize = 1024 * 1024;
        let mut rb = RingBuffer::<usize, N>::new_heap();

        for i in 1..1024 {
            assert_eq!(i - 1, rb.len());
            *rb.push().unwrap() = i;
            assert_eq!(i, rb.len());
        }

        for i in 1..1024 {
            let value = rb.pop().unwrap();
            assert_eq!(i, *value);
        }
    }

    // This test should fail the build, as 3 is not power of two aligned
    // #[test]
    // fn no_pow2() {
    //     let mut rb = RingBuffer::<usize, 3>::new_inline();
    // }
}
