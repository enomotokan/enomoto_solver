#!/bin/bash
# 副作用 (60 秒、1 回ずつ): 拡張の変わった 5 問で 既定 (def)・ROWS_FIRST (rf)・ROWS_FIRST + PROBE_EQUIV_FIX (rfeq)
cd $(dirname $0)
for p in neos-960392 nursesched-sprint02 physiciansched6-2 roll3000 ns1116954; do
./run_cfg.sh $p 60 def
./run_cfg.sh $p 60 rf ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1
./run_cfg.sh $p 60 rfeq ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1 ENOMOTO_MIP_PROBE_EQUIV_FIX=1
done
