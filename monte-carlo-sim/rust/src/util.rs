
pub fn next_multiple_of(num: usize, multiple: usize) -> usize {
    let remainder = num % multiple;
    match remainder {
        0 => num,
        v => num + multiple - v,
    }
}
