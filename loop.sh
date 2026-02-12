#!/usr/bin/env bash
cargo build --release
./target/release/rx "$*"
