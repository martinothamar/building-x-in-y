bindir := ../../target/release
outbin := ${bindir}/dsa

ifndef FN
override FN = dsa::ring_buffer::RingBufferData<T,_>::init
endif

all: build run

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
