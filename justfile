build:
	cargo build --release --all-features --target wasm32-unknown-unknown
	wasm-bindgen --out-dir target/public --target web target/wasm32-unknown-unknown/release/bitoids.wasm
	cp ./assets/* ./target/public -R

run: build
	basic-http-server ./target/public

clean:
	rm -rf ./target/public
