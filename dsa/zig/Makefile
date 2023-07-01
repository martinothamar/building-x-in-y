build:
	zig build -Doptimize=ReleaseSafe

run:
	zig build run -Doptimize=ReleaseSafe

test:
	zig test src/unit_tests.zig

dasm:
	objdump -S -d -M intel zig-out/bin/dsa > zig-out/bin/dsa.asm
