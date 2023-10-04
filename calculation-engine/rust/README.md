# Calculation Engine in Rust

Implementing using Rust

```sh
Benchmarking scalar 8192
Benchmarking scalar 8192: Warming up for 3.0000 s
Benchmarking scalar 8192: Collecting 100 samples in estimated 5.5377 s (35350 iterations)
Benchmarking scalar 8192: Analyzing
scalar 8192             time:   [199.75 µs 199.90 µs 200.05 µs]
slope  [199.75 µs 200.05 µs] R^2            [0.9988127 0.9988075]
mean   [199.80 µs 200.03 µs] std. dev.      [499.79 ns 711.10 ns]
median [199.73 µs 200.08 µs] med. abs. dev. [444.89 ns 707.90 ns]

Benchmarking vectorized 8192
Benchmarking vectorized 8192: Warming up for 3.0000 s
Benchmarking vectorized 8192: Collecting 100 samples in estimated 5.0058 s (863550 iterations)
Benchmarking vectorized 8192: Analyzing
vectorized 8192         time:   [5.7749 µs 5.7758 µs 5.7769 µs]
slope  [5.7749 µs 5.7769 µs] R^2            [0.9999169 0.9999154]
mean   [5.7758 µs 5.7833 µs] std. dev.      [6.5274 ns 28.720 ns]
median [5.7738 µs 5.7755 µs] med. abs. dev. [2.1785 ns 3.8696 ns]
```
