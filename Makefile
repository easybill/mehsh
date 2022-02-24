run_local:
	cargo run --bin mehsh_check example/local.toml --name=foooo

build-linux:
	docker run --platform linux/amd64 -v "$(CURDIR)":/volume -w /volume -e RUSTFLAGS='-C link-args=-s' -t clux/muslrust cargo build --target=x86_64-unknown-linux-musl --release

build: build-linux