all: build

stat: build
	echo "---$$(date)----------------------------------------------" >> ./.measurements/perf-stats.txt
	perf stat -d -d -d -o ./.measurements/perf-stats.txt --append ./bin/Release/net8.0/linux-x64/publish/csharp

build:
	dotnet publish -c Release
