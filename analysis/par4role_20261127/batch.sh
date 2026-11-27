#!/bin/bash
# 伸びない問題 (combo3 s2 で 1T/4T の比が小さい) と対照 (伸びる問題) を、4T と 1T で prof.py にかける。ENOMOTO_PROF_TREE_PAR=1 を付ける。
# bash batch.sh >> batch.out (済んだもの (logs/<問題>_t<N>.log に「solve returned」がある) は飛ばす)
cd "$(dirname "$0")"
P4="nu25-pr12 exp-1-500-5-5 neos-1122047 neos-1171448 mzzv42z dano3_3 mc11 physiciansched6-2 swath3 supportcase7 mcsched CMS750_4 netdiversion"
P1="nu25-pr12 exp-1-500-5-5 neos-1122047 neos-1171448 mzzv42z dano3_3 mc11 physiciansched6-2 swath3"
for th in 4 1; do
  if [ $th = 4 ]; then P=$P4; else P=$P1; fi
  for p in $P; do
    grep -q "solve returned" logs/${p}_t$th.log 2>/dev/null && continue
    PROF_ENV="ENOMOTO_PROF_TREE_PAR=1" python3 prof.py $p $th 600
  done
done
echo BATCH_DONE
