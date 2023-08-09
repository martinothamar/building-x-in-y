bindir := ../../target/release
outbin := ${bindir}/monte-carlo-sim

all: run

stat: build
	echo "---$$(date)----------------------------------------------" >> ./.measurements/perf-stats.txt
	perf stat -d -d -d -o ./.measurements/perf-stats.txt --append $(outbin)

bench:
	taskset -c 2 cargo bench --bench sim -- --verbose

run: build
	taskset -c 2 $(outbin)

toplev: build
	toplev -l3 --core C2 -v --no-desc taskset -c 2 $(outbin)

build:
	cargo build --release

test:
	cargo test && cargo test --release

dasm:
	cargo objdump --release -- \
	-d -S -M intel > $(bindir)/monte-carlo-sim.asm 2> $(bindir)/monte-carlo-sim.asm.log

flamegraph:
	CARGO_PROFILE_RELEASE_DEBUG=true taskset -c 2 cargo flamegraph
