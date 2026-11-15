#!/bin/sh
# ビルドして計測用の場所 (既定 $HOME/cliqfb_pkg、CF_PKG で変えられる) に入れる。既定と試作は同じ wheel (試作は環境変数で入る)
cd "$(dirname "$0")/../.." || exit 1
P=${CF_PKG:-$HOME/cliqfb_pkg}
pip install -q maturin highspy 2>/dev/null
maturin build --release 2>&1 | grep -E "^error|Built wheel" && rm -rf "$P" && pip install --no-deps --target "$P" target/wheels/enomoto_solver-*.whl >/dev/null 2>&1 && echo "installed to $P"
