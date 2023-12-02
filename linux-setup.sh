#!/usr/bin/env bash
rustup target add x86_64-pc-windows-msvc
cargo install xwin
xwin --accept-license splat --output $HOME/.xwin
cargo build --target=x86_64-pc-windows-msvc --release
