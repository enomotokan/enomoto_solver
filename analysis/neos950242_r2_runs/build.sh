#!/bin/sh
cd "$(dirname "$0")/../.." && maturin build --release -o dist 2>&1 | grep -E "^error|Built wheel" ; pip install -q --no-deps --force-reinstall dist/*.whl 2>&1 | grep -v WARNING
true
