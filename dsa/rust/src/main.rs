#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(const_trait_impl)]
#![feature(adt_const_params)]

use crate::ring_buffer::RingBuffer;

pub mod ring_buffer;
mod internal;

fn main() {
    let rb = RingBuffer::<usize, 8>::new();

    println!("{rb:?}");
}
