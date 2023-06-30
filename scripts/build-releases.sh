#!/bin/bash

targets=(x86_64-pc-windows-msvc aarch64-pc-windows-msvc x86_64-apple-darwin aarch64-apple-darwin x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu)

for target in "${targets[@]}"
do
  rustup target add "$target"

  cargo build --release --target "$target"
done

for target in "${targets[@]}"
do
  ORIGINAL_BIN="target/$target/release/debs"

  if [ -f "$ORIGINAL_BIN" ]
  then
    mkdir -p "builds/$target"

    cp "$ORIGINAL_BIN" "builds/$target/debs"

    tar -C "builds/$target" -czvf "${target}.tar.gz" "debs"
  fi
done