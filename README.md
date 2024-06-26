# Building X in Y

This is a repository where I experment with languages and programming across a variety of topics for learning purposes

* Datastructures and algorithms
* Systems/lowlevel programming
* Distributed systems
* Databases
* Anything high performance

Currently interesting in learning more about the following languages

* Rust
* Zig
* C#
* Go

The code is organized such that the top level folders are topics, and first level subfolders are per programming language.
Example (`git ls-tree -r --name-only HEAD | tree --fromfile`):

```
├── dsa
│   ├── c
│   │   ├── seqlock-queue
│   │   │   ├── Makefile
│   │   │   └── ...
│   │   └── spsc
│   │       ├── Makefile
│   │       └── ...
│   ├── rust
│   │   ├── Cargo.toml
│   │   ├── Makefile
│   │   ├── ...
│   └── zig
│       ├── Makefile
│       ├── build.zig
│       ├── ...
```

## Topics

* [Datastructures and algorithms (DSA)](/dsa) - including some leetcode
* [HTTP server](/http-server) - HTTP server from scratch
* [Monte Carlo simulations](/monte-carlo-sim) - simulating a premier league season 100k times. SIMD, performance engineering
* [Calculation Engine](/calculation-engine) - calculatione engine with a builder API for creating formulas while doing vectorized calculations over columns of data
* [fly.io Gossip Glomers](/flyio-gossip-glomers) - distributed systems challengers centered around gossip protocols and consensus
* [Todo API](/todo-api) - production-ready setup for apps/APIs in various languages

## Notes

[Some notes here](/NOTES.md)
