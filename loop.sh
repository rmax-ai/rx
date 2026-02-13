#!/usr/bin/env bash
set -ex

cargo build --release

./target/release/rx "$*"
