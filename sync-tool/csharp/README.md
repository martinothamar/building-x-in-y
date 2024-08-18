# File sync in .NET

A client and server application for syncing files in .NET.
It is not functional, this was mostly an experiment and learning journey
inspired by principles and approaches used by the Tigerbeetle folks.

C# is realistically not a good choice for this.. but we will see how it works out.
Probably won't finish this project in C#.

## Run

Currently not functional. There are some tests

```sh
dotnet test
```

Eventually

```sh
make run
```

Will run a live test on `test/data`.

## Design

Goals:
* Decent performance
  * Low-level (but cross-platform APIs) - `System.Net.Socket`
    * TCP - it fits the problem well (stream based, ordered)
  * Custom/off-heap (aligned) buffer allocations
  * Custom protocol
    * Zero-serialization (constrain to little-endian)
  * Partition files across threads equivalent to CPU cores
* Safe
  * Checksummed header and bodies to ensure identical/consistent copies
  * _NOT_ secure, no built-in encryption, so should be used in secure networks (VPN could be used to tunnel all traffic)
  * Deterministic/pure core (?), abstracted IO such that safety can be thoroughly tested

Client
* AOT-compiled (for fast startup)

Server
* ?

## Test plan

* Unit-tests for important `Core` stuff
* Integration-tests using tmp folders
* Deterministic simulation tests where unlikely (but possible) IO faults are injected
  * Bitrot/bitflips
  * Timeouts
  * High latency
  * Storage failure (to some extent)
