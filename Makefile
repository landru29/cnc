UBUNTU_VERSION:=20.04

release:
	cargo build --release

release-ubuntu-20: UBUNTU_VERSION:=20.04
release-ubuntu-20: release-ubuntu

release-ubuntu-22: UBUNTU_VERSION:=22.04
release-ubuntu-22: release-ubuntu

release-ubuntu-24: UBUNTU_VERSION:=24.04
release-ubuntu-24: release-ubuntu

release-ubuntu-26: UBUNTU_VERSION:=26.04
release-ubuntu-26: release-ubuntu

release-ubuntu:
	docker run --rm -v "$(shell pwd)":/app -e RUSTFLAGS="-A warnings" -e DEBIAN_FRONTEND=noninteractive -w /app ubuntu:$(UBUNTU_VERSION) bash -c "apt update && apt install -y curl build-essential pkg-config tzdata && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable && source ~/.cargo/env && cargo build --release"

build-fluid:
	docker build \
		--target artifacts \
		--output type=local,dest=.devcontainer/esp32/firmware \
		-f  .devcontainer/esp32/Dockerfile.build \
		.devcontainer/esp32

mock-uart:
	cargo run --release --bin mock -- -c testdata/fluidnc.yaml -p /tmp/ttyV0

release-uart:
	sudo fuser -k -TERM /tmp/ttyV0

.PHONY: release mock-uart release-uart release-ubuntu-20 release-ubuntu-22 release-ubuntu-24 release-ubuntu-26 release-ubuntu