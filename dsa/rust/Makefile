bindir := ../../target/release
outbin := ${bindir}/dsa

ifndef FN
override FN = dsa::ring_buffer::RingBuffer<T,_>::new_inline
endif

all: build run

test:
	cargo test --release -- --test-threads=1

build:
	cargo build --release

check:
	cargo check

run:
	$(outbin)

dasm:
	objdump -S -d -M intel ${outbin} > ${bindir}/dsa.asm

dasmfn:
	cargo asm "$(FN)"

clean:
	cargo clean --release
