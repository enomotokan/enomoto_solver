#!/bin/bash
# swath3 の直し `HEUR_ADAPT=1o` が HEUR_ADAPT の効いた問題・害の出た問題を変えないかの確かめ (1 スレッド、種 1〜3)。
# 2 つの設定 (HEUR_ADAPT=1・=1o、どちらも既定に足す) を 2 本並べて回す。結果は res_swath3_ocheck_{1,1o}.jsonl
cd "$(dirname "$0")/../.."
L=analysis/comboharm_20261124
for pt in 10teams:120 neos-1171448:120 CMS750_4:200 misc07:60 blend2:60 markshare_4_0:60; do
  p=${pt%%:*}; tl=${pt##*:}
  for s in 1 2 3; do
    for v in 1 1o; do
      CH_EXTRA="ENOMOTO_MIP_HEUR_ADAPT=$v" python $L/run.py $L/res_swath3_ocheck_$v.jsonl $p 1 def --tl $tl --seed $s | sed "s/^/$v /" &
    done
    wait
  done
done
