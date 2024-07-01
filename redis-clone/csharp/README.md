# Redis clone

Basic redis clone for running `redis-benchmark` CLI GET and SET benchmarks.
Zero-allocation on hot paths apart from actual storage (e.g. when adding keys/values).
Only unmanaged memory, no GC. Is about 2x faster than current Valkey image.

### Prereqs

```sh
sudo apt install redis-tools # Installs redis-cli and redis-benchmark
```

### Benchmark

Running on a AMD Ryzen 5 5600X 6-Core Processor

```sh
redis-benchmark -t ping,get,set --threads 3 -c 128 -n 1000000
```

## This clone

Run

```sh
dotnet run -c Release --project src/
```

```sh
====== PING_INLINE ======
  1000000 requests completed in 3.97 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

0.00% <= 0.1 milliseconds
7.93% <= 0.2 milliseconds
87.61% <= 0.3 milliseconds
98.08% <= 0.4 milliseconds
99.58% <= 0.5 milliseconds
99.89% <= 0.6 milliseconds
99.96% <= 0.7 milliseconds
99.97% <= 0.8 milliseconds
99.98% <= 0.9 milliseconds
99.99% <= 1.0 milliseconds
99.99% <= 1.1 milliseconds
99.99% <= 1.2 milliseconds
99.99% <= 1.3 milliseconds
99.99% <= 1.4 milliseconds
99.99% <= 1.6 milliseconds
99.99% <= 2 milliseconds
100.00% <= 3 milliseconds
100.00% <= 3 milliseconds
252143.22 requests per second

====== PING_BULK ======
  1000000 requests completed in 3.93 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

99.99% <= 1 milliseconds
100.00% <= 2 milliseconds
100.00% <= 4 milliseconds
254647.31 requests per second

====== SET ======
  1000000 requests completed in 3.96 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

99.98% <= 1 milliseconds
100.00% <= 2 milliseconds
100.00% <= 2 milliseconds
252652.86 requests per second

====== GET ======
  1000000 requests completed in 3.92 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

99.98% <= 1 milliseconds
100.00% <= 3 milliseconds
100.00% <= 4 milliseconds
100.00% <= 4 milliseconds
255167.14 requests per second
```

## Valkey image

Run

```sh
docker run --name valkey -d -p 6379:6379 valkey/valkey:7.2.5 --save ""
```

Run `redis-benchmark`

```sh
====== PING_INLINE ======
  1000000 requests completed in 8.50 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

0.00% <= 0.1 milliseconds
0.00% <= 0.2 milliseconds
0.01% <= 0.3 milliseconds
0.02% <= 0.4 milliseconds
0.03% <= 0.5 milliseconds
0.05% <= 0.6 milliseconds
0.14% <= 0.7 milliseconds
0.24% <= 0.8 milliseconds
0.60% <= 0.9 milliseconds
25.85% <= 1.0 milliseconds
90.89% <= 1.1 milliseconds
95.57% <= 1.2 milliseconds
97.78% <= 1.3 milliseconds
98.09% <= 1.4 milliseconds
98.18% <= 1.5 milliseconds
98.20% <= 1.6 milliseconds
98.22% <= 1.7 milliseconds
98.24% <= 1.8 milliseconds
98.31% <= 1.9 milliseconds
98.97% <= 2 milliseconds
99.97% <= 3 milliseconds
99.98% <= 4 milliseconds
99.99% <= 5 milliseconds
99.99% <= 15 milliseconds
99.99% <= 16 milliseconds
100.00% <= 22 milliseconds
100.00% <= 23 milliseconds
100.00% <= 24 milliseconds
100.00% <= 24 milliseconds
117605.55 requests per second

====== PING_BULK ======
  1000000 requests completed in 8.25 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

32.73% <= 1 milliseconds
99.21% <= 2 milliseconds
100.00% <= 3 milliseconds
121182.74 requests per second

====== SET ======
  1000000 requests completed in 8.50 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

10.74% <= 1 milliseconds
98.58% <= 2 milliseconds
100.00% <= 3 milliseconds
100.00% <= 3 milliseconds
117619.38 requests per second

====== GET ======
  1000000 requests completed in 8.50 seconds
  128 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 3

20.62% <= 1 milliseconds
98.88% <= 2 milliseconds
100.00% <= 3 milliseconds
100.00% <= 3 milliseconds
117619.38 requests per second
```
