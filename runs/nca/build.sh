#!/bin/sh
# ビルドして入れ直す
cd "$(dirname "$0")/../.." && maturin build --release -o dist 2>&1 | grep -E "^error|Built wheel" && pip install -q --no-deps --force-reinstall dist/*.whl 2>/dev/null; rm -f dist/*.whl
