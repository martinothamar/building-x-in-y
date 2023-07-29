bindir := ../../target/release/examples
outbin := ${bindir}/server

all: run

run: build
	$(outbin)

build:
	cargo build --release --example server

test:
	cargo test && cargo test --release
