use dsa::RingBuffer;

fn main() {
    let rb = RingBuffer::<usize, 8>::new();

    println!("{rb:?}");
}
