#!/bin/sh

gcc -Wall -Wextra -pedantic -Werror -O3 -march=native -std=c11 -lpthread -o ./bin/spsc main.c && ./bin/spsc
