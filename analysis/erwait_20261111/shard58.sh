#!/bin/bash
# ERWAIT の 58 問の分担 (2026-11-11): 分担セッションで 1 つ回す。def・ew (ENOMOTO_MIP_ERWAIT=sbc) を問題ごとに続けて、種 1・2・3、60 秒・4 スレッド。
# 同じ (問題, 種) の def と ew は同じ分担で続けて回る (問題 i は i % 分担の数 == 分担の番号 の分担)。
# 使い方: bash analysis/erwait_20261111/shard58.sh 分担の数 分担の番号   (リポジトリの頭で。結果は analysis/erwait_20261111/res58_sh<番号>.jsonl)
# 終わったら res58_sh<番号>.jsonl だけをコミットして push する (logs58/ はコミットしない)。ソルバーは同時に 1 つ (ほかの重い仕事と重ねない)。
set -e
N=$1; I=$2
cd "$(git rev-parse --show-toplevel)"
export ERWAIT_PYROOT=${ERWAIT_PYROOT:-/tmp/erwait_py}
python -c "import highspy" 2>/dev/null || pip install highspy -q
python -c "import maturin" 2>/dev/null || pip install maturin -q
if [ ! -d "$ERWAIT_PYROOT/ew" ]; then
  rm -rf /tmp/erwait_whl && maturin build --release -o /tmp/erwait_whl
  pip install --no-deps --target "$ERWAIT_PYROOT/ew" /tmp/erwait_whl/*.whl -q
fi
python analysis/erwait_20261111/run58.py ew def,ew 1,2,3 "$N" "$I"
