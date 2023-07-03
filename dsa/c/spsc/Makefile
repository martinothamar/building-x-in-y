bindir := ./bin
outbin := ${bindir}/spsc

all: build run

build: main.c
	gcc -Wall -Wextra -pedantic -Werror -O3 -march=native -std=c11 -lpthread -o ${outbin} main.c

run:
	$(outbin)

clean:
	rm -rf ${bindir}/*
