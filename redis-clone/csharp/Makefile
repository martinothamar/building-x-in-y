
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
	redis-benchmark -t get,set --threads 8 -c 128 -n 1000000

profile: export DOTNET_EnableEventLog=0
profile: export DOTNET_PerfMapEnabled=1
profile: export DOTNET_EnableWriteXorExecute=0

.PHONY: profile
profile: build
	perf record -F 99 -g -e cycles:u -- ./src/bin/Release/net8.0/RedisClone

profile-analyze:
	perf report -g
