#!/bin/sh
# Install step for Linux and macOS (herdr-plugin.toml [[build]]). Fetches the prebuilt
# binary for this platform from the GitHub release that matches Cargo.toml's version,
# checks its SHA-256, and puts it where herdr runs it. Anything missing or wrong falls
# back to building from source with cargo. Only this install step uses the network;
# the stage never does.
set -u
cd "$(dirname "$0")/.." || exit 1
build() {
    echo "herdr-fantasy: building from source with cargo" >&2
    exec cargo build --release --locked
}
ver=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)
case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) target=x86_64-unknown-linux-gnu ;;
    Darwin-arm64) target=aarch64-apple-darwin ;;
    Darwin-x86_64) target=x86_64-apple-darwin ;;
    *) target="" ;;
esac
[ -n "$target" ] && [ -n "$ver" ] || build
name="herdr-fantasy-$target"
base="https://github.com/KantaKan/herdr-fantasy/releases/download/v$ver"
fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL -o "$2" "$1"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$2" "$1"
    else
        return 1
    fi
}
tmp=$(mktemp -d 2>/dev/null) || build
if fetch "$base/$name" "$tmp/bin" && fetch "$base/$name.sha256" "$tmp/sum"; then
    want=$(cut -d ' ' -f 1 "$tmp/sum")
    got=$( (sha256sum "$tmp/bin" 2>/dev/null || shasum -a 256 "$tmp/bin") | cut -d ' ' -f 1)
    if [ -n "$want" ] && [ "$want" = "$got" ]; then
        mkdir -p target/release
        mv "$tmp/bin" target/release/herdr-fantasy
        chmod +x target/release/herdr-fantasy
        rm -rf "$tmp"
        echo "herdr-fantasy: installed the prebuilt $name v$ver"
        exit 0
    fi
    echo "herdr-fantasy: checksum mismatch for $name" >&2
else
    echo "herdr-fantasy: no prebuilt $name for v$ver" >&2
fi
rm -rf "$tmp"
build
