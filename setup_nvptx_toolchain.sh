#!/bin/bash
set -xue

NIGHTLY=nightly-2020-09-20
rustup toolchain add ${NIGHTLY}
rustup component add rustfmt --toolchain ${NIGHTLY}
rustup target add nvptx64-nvidia-cuda --toolchain ${NIGHTLY}
cargo install ptx-linker -f
