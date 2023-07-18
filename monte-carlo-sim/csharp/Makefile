all: run

stat: build
	echo "---$$(date)----------------------------------------------" >> ./.measurements/perf-stats.txt
	perf stat -d -d -d -o ./.measurements/perf-stats.txt --append dotnet run -c Release --no-build --no-restore

run: build
	dotnet run -c Release --no-build --no-restore

build:
	dotnet build -c Release
