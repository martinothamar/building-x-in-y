# Datastructures and algorithms - Rust

## Ringbuffer

```
RingBuffers/dsa::ring_buffer::RingBuffer heap 64
                        time:   [71.760 ns 72.115 ns 72.525 ns]
Found 12 outliers among 100 measurements (12.00%)
  4 (4.00%) high mild
  8 (8.00%) high severe

RingBuffers/dsa::ring_buffer::RingBuffer inline 64
                        time:   [59.257 ns 59.550 ns 59.902 ns]
Found 11 outliers among 100 measurements (11.00%)
  5 (5.00%) high mild
  6 (6.00%) high severe

RingBuffers/std::collections::VecDeque 64
                        time:   [124.11 ns 125.14 ns 126.50 ns]
Found 12 outliers among 100 measurements (12.00%)
  9 (9.00%) high mild
  3 (3.00%) high severe
```
