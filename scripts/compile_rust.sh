#!/bin/sh

cargo build --release --manifest-path ./monistor-rs/Cargo.toml

mkdir -p ./_build/lib

# cp ./monistor-rs/target/release/monistord ./_build/lib/monistord
cp ./monistor-rs/target/release/monistord ~/bin/monistord