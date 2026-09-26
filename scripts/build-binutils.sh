#!/bin/sh
# Build the independent oracle, never a library dependency.
set -eu
version=2.47
sha256=154ab23b60070e8f27013c22977f1129425d67d1e8acd6e13010e617811e4cff
root=${1:-target/h8-binutils}
mkdir -p "$root"
root=$(cd "$root" && pwd)
archive="$root/binutils-$version.tar.xz"
if [ ! -f "$archive" ]; then
    if [ -n "${H8_BINUTILS_ARCHIVE:-}" ]; then
        cp "$H8_BINUTILS_ARCHIVE" "$archive"
    else
        # A failed download must not become a cached archive. Both upstream
        # locations serve the same release; the pinned digest below is mandatory.
        downloaded=false
        for base in https://ftp.gnu.org/gnu/binutils https://sourceware.org/pub/binutils/releases; do
            if curl --fail --silent --show-error --location --retry 1 \
                --connect-timeout 15 --max-time 180 \
                "$base/binutils-$version.tar.xz" -o "$archive.part"; then
                mv "$archive.part" "$archive"
                downloaded=true
                break
            fi
        done
        if [ "$downloaded" != true ]; then
            rm -f "$archive.part"
            echo "cannot download the pinned binutils release" >&2
            exit 1
        fi
    fi
fi
actual=$(shasum -a 256 "$archive")
actual=${actual%% *}
if [ "$actual" != "$sha256" ]; then
    echo "binutils archive checksum mismatch: $archive" >&2
    exit 1
fi
if [ ! -d "$root/binutils-$version" ]; then
    tar -xf "$archive" -C "$root"
fi
mkdir -p "$root/build"
cd "$root/build"
if [ ! -f Makefile ]; then
    "$root/binutils-$version/configure" --target=h8300-elf \
        --prefix="$root/install" --disable-nls --disable-werror \
        --disable-gdb --disable-sim --disable-gprof --disable-gprofng \
        --disable-gold --disable-libctf --disable-zstd --without-debuginfod
fi
make -j "${H8_BUILD_JOBS:-2}" all-gas all-ld all-binutils
make install-gas install-ld install-binutils
"$root/install/bin/h8300-elf-as" --version
"$root/install/bin/h8300-elf-ld" --version
