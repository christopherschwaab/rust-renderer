#!/usr/bin/env bash
readonly CWD="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cd "$CWD"

rustup target add x86_64-pc-windows-msvc
cargo install xwin
xwin --accept-license splat --output $HOME/.xwin
cargo build --target=x86_64-pc-windows-msvc --release

mkdir -p .cargo
if [ .cargo/config ]; then
    echo "Skipping .cargo/config update because it already exists..."
else
    echo "Updating .cargo/config with default target and x86_64-pc-windows-msvc linker flags..."
    cat - <<EOF > .cargo/config
[target.x86_64-pc-windows-msvc]
linker = "lld"
rustflags = [
  "-Lnative=$HOME/.xwin/crt/lib/x86_64",
  "-Lnative=$HOME/.xwin/sdk/lib/um/x86_64",
  "-Lnative=$HOME/.xwin/sdk/lib/ucrt/x86_64"
]

[build]
target = "x86_64-pc-windows-msvc"
EOF
fi
