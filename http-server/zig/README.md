# Zig HTTP server

Uses IOUring and ring provided buffers to have very efficient network IO.
Following advice from: https://github.com/axboe/liburing/wiki/io_uring-and-networking-in-2023

Compared against the [faf TechEmpower bencharm](https://github.com/TechEmpower/FrameworkBenchmarks/tree/b76dbf54842a6fb9c4f7ae49e12206d80b73339f/frameworks/Rust/faf) (winner as of 2024).

### Prereqs

A newer Linux kernel, 6.10+

```sh
brew install zig zls
```

### Results

FAF:

```sh
$ docker run -p 8081:8080 -d --name faf faf:latest
$ oha -n 1000000 -c 500 http://127.0.0.1:8081/plaintext
Summary:
  Success rate:	100.00%
  Total:	2.8295 secs
  Slowest:	0.0947 secs
  Fastest:	0.0000 secs
  Average:	0.0014 secs
  Requests/sec:	353417.2163

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	4.38 MiB

Response time histogram:
  0.000 [1]      |
  0.009 [998826] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.019 [623]    |
  0.028 [59]     |
  0.038 [0]      |
  0.047 [29]     |
  0.057 [36]     |
  0.066 [23]     |
  0.076 [120]    |
  0.085 [208]    |
  0.095 [75]     |

Response time distribution:
  10.00% in 0.0005 secs
  25.00% in 0.0008 secs
  50.00% in 0.0012 secs
  75.00% in 0.0018 secs
  90.00% in 0.0024 secs
  95.00% in 0.0029 secs
  99.00% in 0.0041 secs
  99.90% in 0.0128 secs
  99.99% in 0.0814 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0015 secs, 0.0002 secs, 0.0150 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0038 secs

Status code distribution:
  [200] 1000000 responses

```

This code:

```sh
$ zig build run -Doptimize=ReleaseSafe
$ oha -n 1000000 -c 500 http://127.0.0.1:8080/plaintext
Summary:
  Success rate:	100.00%
  Total:	3.9885 secs
  Slowest:	0.0281 secs
  Fastest:	0.0011 secs
  Average:	0.0020 secs
  Requests/sec:	250721.7318

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	3.11 MiB

Response time histogram:
  0.001 [1]      |
  0.004 [996005] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.006 [3637]   |
  0.009 [95]     |
  0.012 [0]      |
  0.015 [0]      |
  0.017 [0]      |
  0.020 [2]      |
  0.023 [244]    |
  0.025 [13]     |
  0.028 [3]      |

Response time distribution:
  10.00% in 0.0016 secs
  25.00% in 0.0017 secs
  50.00% in 0.0020 secs
  75.00% in 0.0021 secs
  90.00% in 0.0023 secs
  95.00% in 0.0025 secs
  99.00% in 0.0031 secs
  99.90% in 0.0047 secs
  99.99% in 0.0216 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0038 secs, 0.0001 secs, 0.0239 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0033 secs

Status code distribution:
  [200] 1000000 responses
```