all:
	dotnet publish -c Release && /usr/lib/linux-tools/5.15.0-76-generic/perf record -F 99 -e branches,branch-misses,cache-misses ./bin/Release/net8.0/linux-x64/publish/csharp
