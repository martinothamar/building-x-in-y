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

* [Datastructures and algorithms (DSA)](/dsa)
* [HTTP server](/http-server)
* [Monte Carlo simulations](/monte-carlo-sim) - simulating a premier league season 100k times. SIMD

## Notes

[Some notes here](/NOTES.md)
