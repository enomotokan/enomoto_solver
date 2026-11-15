#!/bin/bash
# COMP07 の 58 問の分担 (2026-11-11): 分担セッションで 1 つ回す。def・ebiR・ebiRK を問題ごとに続けて、種 1・2・3、60 秒・4 スレッド。
# 使い方: bash analysis/comp07_20261111/shard58.sh 分担の数 分担の番号   (リポジトリの頭で。結果は analysis/comp07_20261111/res58_sh<番号>.jsonl)
# 終わったら res58_sh<番号>.jsonl だけをコミットして push する (logs58/ はコミットしない)。
set -e
N=$1; I=$2
cd "$(git rev-parse --show-toplevel)"
export COMP07_PYROOT=${COMP07_PYROOT:-/tmp/comp07_py}
python -c "import highspy" 2>/dev/null || pip install highspy maturin -q
python -c "import maturin" 2>/dev/null || pip install maturin -q
if [ ! -d "$COMP07_PYROOT/k" ]; then
  rm -rf /tmp/comp07_whl && maturin build --release -o /tmp/comp07_whl
  pip install --no-deps --target "$COMP07_PYROOT/k" /tmp/comp07_whl/*.whl -q
fi
python analysis/comp07_20261111/run58.py k def,ebiR,ebiRK 1,2,3 "$N" "$I"
