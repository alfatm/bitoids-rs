cargo serve --release

or

cargo install basic-http-server

mkdir -p public
cp ./assets/\* ./public
basic-http-server ./public

or

cargo build --release --all-features --target wasm32-unknown-unknown
