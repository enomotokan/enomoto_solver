#!/bin/sh
# リリースのビルドを入れ直す (計測の前に。リポジトリの根で maturin を使う)
# target/wheels に古い wheel が残っていると glob が複数に当たり入れ直しに失敗するので、いちばん新しい wheel だけを入れる。
cd "$(dirname "$0")/../.." && pip install -q maturin highspy==1.15.1 2>/dev/null; maturin build --release 2>&1 | grep -E "^error|Built wheel" && W=$(ls -t target/wheels/enomoto_solver-*.whl | head -n 1) && pip install --force-reinstall --no-deps "$W" >/dev/null 2>&1 && echo "installed $W"
