release:
	cargo build --release

mock-uart:
	cargo run --release --bin mock -- -c testdata/mock.yaml -p /tmp/ttyV0

release-uart:
	sudo fuser -k -TERM /tmp/ttyV0

.PHONY: release mock-uart release-uart