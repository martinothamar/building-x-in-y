bindir := ../../target/release
outbin := ${bindir}/monte-carlo-sim

all: run

stat: build
	echo "---$$(date)----------------------------------------------" >> ./.measurements/perf-stats.txt
	perf stat -d -d -d -o ./.measurements/perf-stats.txt --append $(outbin)

bench:
	cargo bench --bench sim -- --verbose

run: build
	$(outbin)

build:
	cargo build --release

test:
	cargo test

dasm:
	cargo objdump --release -- \
	-d -S -M intel > $(bindir)/monte-carlo-sim.asm 2> $(bindir)/monte-carlo-sim.asm.log

flamegraph:
	CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph
