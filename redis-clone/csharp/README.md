# Redis clone

Basic redis clone for running `redis-benchmark` CLI GET and SET benchmarks

### Benchmark

```sh
redis-benchmark -t get,set --threads 4 -c 4 -n 1000000
```

## This clone

Run

```sh
dotnet run -c Release
```

```sh
====== SET ======
  1000000 requests completed in 4.00 seconds
  4 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 4

99.99% <= 0.1 milliseconds
99.99% <= 0.2 milliseconds
100.00% <= 0.3 milliseconds
100.00% <= 0.4 milliseconds
100.00% <= 0.6 milliseconds
100.00% <= 0.7 milliseconds
100.00% <= 1.0 milliseconds
100.00% <= 1.1 milliseconds
100.00% <= 1.1 milliseconds
249875.08 requests per second

====== GET ======
  1000000 requests completed in 4.00 seconds
  4 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 4

99.98% <= 0.1 milliseconds
99.99% <= 0.2 milliseconds
99.99% <= 0.3 milliseconds
100.00% <= 0.4 milliseconds
100.00% <= 0.5 milliseconds
100.00% <= 0.7 milliseconds
100.00% <= 0.8 milliseconds
100.00% <= 0.9 milliseconds
100.00% <= 1.0 milliseconds
100.00% <= 1.1 milliseconds
100.00% <= 2 milliseconds
249875.08 requests per second
```

## Valkey image

Run

```sh
docker run --name valkey -d -p 6379:6379 valkey/valkey:7.2.5 --save ""
```

Run `redis-benchmark`

```sh
====== SET ======
  1000000 requests completed in 8.00 seconds
  4 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 4

99.99% <= 0.1 milliseconds
100.00% <= 0.2 milliseconds
100.00% <= 0.3 milliseconds
100.00% <= 0.4 milliseconds
100.00% <= 0.4 milliseconds
124984.37 requests per second

====== GET ======
  1000000 requests completed in 8.00 seconds
  4 parallel clients
  3 bytes payload
  keep alive: 1
  host configuration "save":
  host configuration "appendonly": no
  multi-thread: yes
  threads: 4

99.98% <= 0.1 milliseconds
99.99% <= 0.2 milliseconds
100.00% <= 0.3 milliseconds
100.00% <= 0.4 milliseconds
100.00% <= 0.5 milliseconds
100.00% <= 0.6 milliseconds
100.00% <= 0.7 milliseconds
100.00% <= 0.8 milliseconds
100.00% <= 0.9 milliseconds
100.00% <= 1.0 milliseconds
100.00% <= 1.1 milliseconds
100.00% <= 1.8 milliseconds
100.00% <= 1.9 milliseconds
100.00% <= 2 milliseconds
100.00% <= 2 milliseconds
124968.76 requests per second
```
