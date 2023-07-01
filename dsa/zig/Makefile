build:
	zig build -Doptimize=ReleaseSafe

test:
	zig test src/RingBuffer.zig

dasm:
	objdump -S -d -M intel zig-out/bin/dsa > zig-out/bin/dsa.asm
