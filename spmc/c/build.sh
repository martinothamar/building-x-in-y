#!/bin/sh

gcc -Wall -Wextra -pedantic -Werror -O3 -std=c11 -lpthread -o ./bin/spmc main.c && ./bin/spmc
