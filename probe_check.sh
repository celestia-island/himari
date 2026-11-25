#!/bin/sh
cd packages/check/fixtures/probe || exit 1
# The log path is sandbox-internal (this script is only run inside the
# himari sandbox lane); outside the sandbox it still works — the log lands
# wherever /sandbox points on the host, and the exit status is what matters.
CARGO_TARGET_DIR="${CARGO_TARGET_DIR}/probe" cargo build --bins --keep-going --offline > /sandbox/src/probe-build.log 2>&1
echo "exit=$?"
