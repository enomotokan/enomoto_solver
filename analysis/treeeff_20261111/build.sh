#!/bin/sh
# リリースのビルドを入れ直す (計測の前に。リポジトリの根で maturin を使う)
cd "$(dirname "$0")/../.." && pip install -q maturin highspy==1.15.1 2>/dev/null; maturin build --release 2>&1 | grep -E "^error|Built wheel" && pip install --force-reinstall --no-deps target/wheels/enomoto_solver-*.whl >/dev/null 2>&1 && echo installed
