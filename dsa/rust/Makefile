bindir := ../../target/release
outbin := ${bindir}/dsa

all: build run

build:
	cargo build --release

check:
	cargo check

run:
	$(outbin)

clean:
	cargo clean --release
