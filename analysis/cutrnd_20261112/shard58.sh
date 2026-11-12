#!/bin/bash
# CUTRND の 58 問の分担 (2026-11-12): 分担セッションで 1 つ回す。def・cr (ENOMOTO_MIP_CUTRND=esf) を問題ごとに続けて、種 1・2・3、60 秒・4 スレッド。
# 同じ (問題, 種) の def と cr は同じ分担で続けて回る (問題 i は i % 分担の数 == 分担の番号 の分担)。
# 使い方: bash analysis/cutrnd_20261112/shard58.sh 分担の数 分担の番号   (リポジトリの頭で。結果は analysis/cutrnd_20261112/res58_sh<番号>.jsonl)
# 終わったら res58_sh<番号>.jsonl だけをコミットして push する (logs58/ はコミットしない)。ソルバーは同時に 1 つ (ほかの重い仕事と重ねない)。
set -e
N=$1; I=$2
cd "$(git rev-parse --show-toplevel)"
export CUTRND_PYROOT=${CUTRND_PYROOT:-/tmp/cutrnd_py}
python -c "import highspy" 2>/dev/null || pip install highspy==1.15.1 -q
python -c "import maturin" 2>/dev/null || pip install maturin -q
if [ ! -d "$CUTRND_PYROOT/a" ]; then
  rm -rf /tmp/cutrnd_whl && maturin build --release -o /tmp/cutrnd_whl
  pip install --no-deps --target "$CUTRND_PYROOT/a" /tmp/cutrnd_whl/*.whl -q
fi
python analysis/cutrnd_20261112/run58.py a def,cr 1,2,3 "$N" "$I"
