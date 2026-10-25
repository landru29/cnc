release:
	cargo build --release

build-fluid:
	docker build \
		--target artifacts \
		--output type=local,dest=.devcontainer/esp32/firmware \
		-f  .devcontainer/esp32/Dockerfile.build \
		.devcontainer/esp32

# 	docker build -t fluid-build -f  $(shell pwd)/.devcontainer/esp32/Dockerfile.build .devcontainer/esp32
# 	docker run \
# 		-v $(shell pwd)/.devcontainer/esp32/firmware:/out \
# 		fluid-build

mock-uart:
	cargo run --release --bin mock -- -c testdata/fluidnc.yaml -p /tmp/ttyV0

release-uart:
	sudo fuser -k -TERM /tmp/ttyV0

.PHONY: release mock-uart release-uart