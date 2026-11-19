#!/bin/sh
# P2-INF: PROWTINY で遅く出た 10teams・noswot を 4 スレッド 60 秒 × 種 1〜8 × {def, rowtiny} で回し直す (道の運かを見る)。
# リポジトリの根から: PYTHONPATH=<試作のビルド> sh analysis/p2inf_20261119/rep4.sh → res_rep4.jsonl
D=analysis/p2inf_20261119
for p in 10teams noswot; do f=$(ls .miplib_cache/$p.mps* | head -1)
 for s in 1 2 3 4 5 6 7 8; do for c in def rowtiny; do
  E=""; [ $c = rowtiny ] && E=ENOMOTO_MIP_LPINF_ROWTINY=1
  r=$(env $E ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SEED=$s python3 scripts/run_miplib_benchmark.py --worker-ours $f --time-limit 60 --threads 4 2>/dev/null | grep '^{' | tail -1)
  echo "{\"problem\":\"$p\",\"cfg\":\"$c\",\"seed\":$s,\"result\":${r:-null}}" >> $D/res_rep4.jsonl
 done; done; done
