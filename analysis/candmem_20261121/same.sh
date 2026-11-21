#!/bin/sh
# 直し (分離の tableau 行の流し込み) の前後で、根のカットのループの各ラウンドの候補が同じかの確かめ。
# 使い方: sh same.sh <old の展開先> <new の展開先> <名札> [問題…]  (4 スレッド・30 秒、ENOMOTO_MIP_DEBUG_DUMP_CANDS で候補を書き出す)
# 出力は same/<問題>/<名札>_<old|new>/cands_r<k>.txt。比べは same_cmp.py
cd "$(dirname "$0")"
R=$(cd ../.. && pwd)
OLD=$1; NEW=$2; TAG=$3; shift 3
for p in $*; do
  for b in old new; do
    d=same/$p/${TAG}_$b; mkdir -p $d; rm -f $d/*.txt
    if [ $b = old ]; then PP=$OLD; else PP=$NEW; fi
    env PYTHONPATH=$PP ENOMOTO_MIP_DEBUG_DUMP_CANDS=$d python "$R/analysis/smallprob_overhead_20261028/prof_run.py" ours "$R/.miplib_cache/$p.mps" $d/run.log --threads 4 --time-limit ${TL:-30} >/dev/null 2>&1
    echo "$p $b $(tail -1 $d/run.log | cut -c1-150)"
  done
done
