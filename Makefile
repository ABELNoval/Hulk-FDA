.PHONY: build clean

build:
	cargo build --release --features llvm-verify
	cp target/release/hulk-compiler ./hulk

clean:
	cargo clean
	rm -f ./hulk
