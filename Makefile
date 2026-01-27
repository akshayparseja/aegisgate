.PHONY: build run test clean

build:
	cargo build

run:
	cargo run --bin aegis-proxy

test:
	cargo test

clean:
	cargo clean
