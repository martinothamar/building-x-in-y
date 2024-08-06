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
  Total:	3.0000 secs
  Slowest:	0.0225 secs
  Fastest:	0.0000 secs
  Average:	0.0015 secs
  Requests/sec:	333329.7771

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	4.13 MiB

Response time histogram:
  0.000 [1]      |
  0.002 [838529] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.005 [154601] |■■■■■
  0.007 [6244]   |
  0.009 [221]    |
  0.011 [116]    |
  0.014 [2]      |
  0.016 [32]     |
  0.018 [72]     |
  0.020 [150]    |
  0.022 [32]     |

Response time distribution:
  10.00% in 0.0005 secs
  25.00% in 0.0008 secs
  50.00% in 0.0014 secs
  75.00% in 0.0020 secs
  90.00% in 0.0026 secs
  95.00% in 0.0031 secs
  99.00% in 0.0043 secs
  99.90% in 0.0059 secs
  99.99% in 0.0190 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0051 secs, 0.0001 secs, 0.0203 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0026 secs

Status code distribution:
  [200] 1000000 responses
```

This code:

```sh
$ zig build run -Doptimize=ReleaseSafe
$ oha -n 1000000 -c 500 http://127.0.0.1:8080/plaintext
Summary:
  Success rate:	100.00%
  Total:	2.0638 secs
  Slowest:	0.0359 secs
  Fastest:	0.0000 secs
  Average:	0.0010 secs
  Requests/sec:	484546.2762

  Total data:	12.40 MiB
  Size/request:	13 B
  Size/sec:	6.01 MiB

Response time histogram:
  0.000 [1]      |
  0.004 [987194] |■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■■
  0.007 [12234]  |
  0.011 [55]     |
  0.014 [5]      |
  0.018 [262]    |
  0.022 [175]    |
  0.025 [0]      |
  0.029 [0]      |
  0.032 [34]     |
  0.036 [40]     |

Response time distribution:
  10.00% in 0.0003 secs
  25.00% in 0.0004 secs
  50.00% in 0.0008 secs
  75.00% in 0.0013 secs
  90.00% in 0.0021 secs
  95.00% in 0.0027 secs
  99.00% in 0.0038 secs
  99.90% in 0.0055 secs
  99.99% in 0.0201 secs


Details (average, fastest, slowest):
  DNS+dialup:	0.0043 secs, 0.0001 secs, 0.0205 secs
  DNS-lookup:	0.0000 secs, 0.0000 secs, 0.0031 secs

Status code distribution:
  [200] 1000000 responses
```