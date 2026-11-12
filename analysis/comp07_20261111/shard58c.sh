#!/bin/bash
# COMP07 の 58 問の分担 (2026-11-11): 分担セッションで 1 つ回す。def・lmboost2 (LMBOOST の 2 版) を問題ごとに続けて、種 1・2・3、60 秒・4 スレッド。
# 使い方: bash analysis/comp07_20261111/shard58c.sh 分担の数 分担の番号   (リポジトリの頭で。結果は analysis/comp07_20261111/res58c_sh<番号>.jsonl)
# 終わったら res58c_sh<番号>.jsonl だけをコミットして push する (logs58/ はコミットしない)。
set -e
N=$1; I=$2
cd "$(git rev-parse --show-toplevel)"
export COMP07_PYROOT=${COMP07_PYROOT:-/tmp/comp07_py}
python -c "import highspy" 2>/dev/null || pip install highspy maturin -q
python -c "import maturin" 2>/dev/null || pip install maturin -q
if [ ! -d "$COMP07_PYROOT/lb2" ]; then
  rm -rf /tmp/comp07_whl_lb2 && maturin build --release -o /tmp/comp07_whl_lb2
  pip install --no-deps --target "$COMP07_PYROOT/lb2" /tmp/comp07_whl_lb2/*.whl -q
fi
COMP07_TAG=c python analysis/comp07_20261111/run58.py lb2 def,lmboost2 1,2,3 "$N" "$I"
