.PHONY: test
test:
	zig test src/root.zig

.PHONY: build
build:
	zig build -Doptimize=ReleaseSafe

.PHONY: run
run: build
	./zig-out/bin/zserv
