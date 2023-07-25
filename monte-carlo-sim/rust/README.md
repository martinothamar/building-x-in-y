# Monte Carlo simulations in Rust

This is the main implementation of the project described in [the parent folder README](/monte-carlo-sim/).

## Run

```bash
make
```

## Disassembly

```bash
# Prereqs
cargo install cargo-binutils
rustup component add llvm-tools-preview

make dasm
```

## Stats

```bash
make stat
```

## Flamegraph

```bash
make flamegraph
```


