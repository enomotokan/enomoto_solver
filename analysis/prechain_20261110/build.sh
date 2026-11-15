#!/bin/sh
# ビルドして入れ直す (計測の前に)
cd "$(dirname "$0")/../.." && maturin build --release 2>&1 | grep -E "^error|Built wheel" && pip install --force-reinstall --no-deps target/wheels/enomoto_solver-*.whl >/dev/null 2>&1 && echo installed
