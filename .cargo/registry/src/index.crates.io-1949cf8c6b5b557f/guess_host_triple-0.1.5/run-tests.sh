#!/bin/sh
set -eu

cargo doc
cargo test

TARGETS="
	i686-unknown-linux-gnu
	x86_64-unknown-linux-gnu
	x86_64-pc-windows-gnu
	"

RUST_LOG="guess_host_triple=debug"
export RUST_LOG

# Test 32-bit Linux

cargo build --example guess-host-triple --target i686-unknown-linux-gnu

OUTPUT=$(target/i686-unknown-linux-gnu/debug/examples/guess-host-triple)

# We expect 64-bit output because we're running on 64-bit Linux
if [ "$OUTPUT" != "x86_64-unknown-linux-gnu" ]; then
	echo "library is broken, printed $OUTPUT"
	exit 1
fi

# Test 64-bit Linux
cargo build --example guess-host-triple --target x86_64-unknown-linux-gnu

OUTPUT=$(target/x86_64-unknown-linux-gnu/debug/examples/guess-host-triple)

if [ "$OUTPUT" != "x86_64-unknown-linux-gnu" ]; then
	echo "library is broken, printed $OUTPUT"
	exit 1
fi

# Test 64-bit Windows
RUSTFLAGS="-C linker=x86_64-w64-mingw32-gcc" \
	cargo build --example guess-host-triple \
	--target x86_64-pc-windows-gnu
WINEPREFIX=$(mktemp -p /tmp -d wineprefix.XXXXXXXX)
WINEARCH=win64

export RUSTFLAGS WINEPREFIX WINEARCH

OUTPUT=$(wine target/x86_64-pc-windows-gnu/debug/examples/guess-host-triple)

# Windows always defaults to MSVC target.
if [ "$OUTPUT" != "x86_64-pc-windows-msvc" ]; then
	echo "library is broken, printed $OUTPUT"
	exit 1
fi

rm -rf "$WINEPREFIX"
