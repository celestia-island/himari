#!/bin/sh
cd packages/check/fixtures/probe || exit 1
CARGO_TARGET_DIR="${CARGO_TARGET_DIR}/probe" cargo build --bins --keep-going --offline > /sandbox/src/probe-build.log 2>&1
echo "exit=$?"
