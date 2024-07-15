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
  Total:	2.9157 secs
  Slowest:	0.0356 secs
  Fastest:	0.0000 secs
  Average:	0.0014 secs
  Requests/sec:	342971.0068

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	4.25 MiB

Response time histogram:
  0.000 [1]      |
  0.004 [977173] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.007 [22287]  |
  0.011 [220]    |
  0.014 [14]     |
  0.018 [0]      |
  0.021 [105]    |
  0.025 [164]    |
  0.028 [22]     |
  0.032 [1]      |
  0.036 [13]     |

Response time distribution:
  10.00% in 0.0005 secs
  25.00% in 0.0008 secs
  50.00% in 0.0013 secs
  75.00% in 0.0019 secs
  90.00% in 0.0025 secs
  95.00% in 0.0030 secs
  99.00% in 0.0042 secs
  99.90% in 0.0058 secs
  99.99% in 0.0230 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0083 secs, 0.0002 secs, 0.0227 secs
  DNS-lookup:	0.0001 secs, 0.0000 secs, 0.0064 secs

Status code distribution:
  [200] 1000000 responses
```

This code:

```sh
$ zig build run -Doptimize=ReleaseSafe
$ oha -n 1000000 -c 500 http://127.0.0.1:8080/plaintext
Summary:
  Success rate:	100.00%
  Total:	1.4204 secs
  Slowest:	0.0230 secs
  Fastest:	0.0000 secs
  Average:	0.0007 secs
  Requests/sec:	704021.0209

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	8.73 MiB

Response time histogram:
  0.000 [1]      |
  0.002 [982496] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.005 [16709]  |
  0.007 [531]    |
  0.009 [1]      |
  0.012 [0]      |
  0.014 [0]      |
  0.016 [0]      |
  0.018 [0]      |
  0.021 [199]    |
  0.023 [63]     |

Response time distribution:
  10.00% in 0.0003 secs
  25.00% in 0.0004 secs
  50.00% in 0.0005 secs
  75.00% in 0.0009 secs
  90.00% in 0.0014 secs
  95.00% in 0.0017 secs
  99.00% in 0.0027 secs
  99.90% in 0.0044 secs
  99.99% in 0.0203 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0067 secs, 0.0004 secs, 0.0207 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0033 secs

Status code distribution:
  [200] 1000000 responses
```