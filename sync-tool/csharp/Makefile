.PHONY: build-client
build-client:
	dotnet publish -r linux-x64 -c Release -o bin/ src/Sync.Client/
	mv bin/Sync.Client bin/sync-client

.PHONY: run-client
run-client: build-client
	./bin/sync-client --dir test/data/ --port 8080
