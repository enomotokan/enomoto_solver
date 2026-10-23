#!/bin/bash
# wheel を作って入れ直す
cd "$(dirname "$0")/../.."
rm -f dist/*.whl
maturin build --release -o dist 2>&1 | tail -1
pip install -q --no-deps --force-reinstall dist/*.whl 2>&1 | grep -v WARNING
