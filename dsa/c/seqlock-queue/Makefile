bindir := ./bin
outbin := ${bindir}/seqlock-queue

all: build run

build: main.c
	gcc -Wall -Wextra -pedantic -Werror -O3 -g3 -std=c11 -lpthread -o ${outbin} main.c

run:
	$(outbin)

clean:
	rm -rf ${bindir}/*
