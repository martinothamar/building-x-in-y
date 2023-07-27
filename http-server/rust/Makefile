bindir := ../../target/release
outbin := ${bindir}/httpsrv

all: run

run: build
	$(outbin)

build:
	cargo build --release

test:
	cargo test && cargo test --release
