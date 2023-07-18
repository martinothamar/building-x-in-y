bindir := ../../target/release
outbin := ${bindir}/monte-carlo-sim

all: run

stat: build
	echo "---$$(date)----------------------------------------------" >> ./.measurements/perf-stats.txt
	perf stat -d -d -d -o ./.measurements/perf-stats.txt --append $(outbin)

run: build
	$(outbin)

build:
	cargo build --release

dasm:
	cargo objdump --release -- \
	-d -S -M intel > $(bindir)/monte-carlo-sim.asm 2> $(bindir)/monte-carlo-sim.asm.log
