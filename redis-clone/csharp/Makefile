
.PHONY: setup
setup:
	sudo apt install redis-tools
	dotnet tool install --global JetBrains.dotTrace.GlobalTools

.PHONY: run
run:
	dotnet run -c Release --project src/

.PHONY: build
build:
	dotnet build -c Release

.PHONY: bench
bench:
	redis-benchmark -t ping,get,set --threads 3 -c 128 -n 1000000

.PHONY: profile
profile:
	dottrace start --profiling-type=Timeline --framework=NetCore ./src/bin/Release/net8.0/RedisClone
