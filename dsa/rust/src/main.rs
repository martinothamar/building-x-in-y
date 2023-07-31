#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(const_trait_impl)]
#![feature(adt_const_params)]
#![feature(new_uninit)]

use crate::ring_buffer::RingBuffer;

mod internal;
pub mod list;
pub mod ring_buffer;

fn main() {
    let mut rb = RingBuffer::<usize, 8>::new_heap();

    let mut value: usize;

    println!("{rb:?}");
    *rb.push().unwrap() = 1;
    println!("{rb:?}");
    value = *rb.pop().unwrap();
    println!("{rb:?} - {value}");

    *rb.push().unwrap() = 2;
    println!("{rb:?}");
    *rb.push().unwrap() = 3;
    println!("{rb:?}");
    *rb.push().unwrap() = 4;
    println!("{rb:?}");

    value = *rb.pop().unwrap();
    println!("{rb:?} - {value}");
    value = *rb.pop().unwrap();
    println!("{rb:?} - {value}");
    value = *rb.pop().unwrap();
    println!("{rb:?} - {value}");
}
