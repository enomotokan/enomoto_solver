#!/bin/bash
# 手当ての形 D' (D + PROBE_MIN_GAIN + CLIQUE_EXTEND_MIN_FRAC) と既定を 10 問・4 回ずつ (順を入れ替え)、bench_mix.sh と同じ呼び出し
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_SHA=dae7e38
DP="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1 ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3 ENOMOTO_MIP_CUTSEL_PROBE_ALT=NS ENOMOTO_T_MIP_CUTSEL_PROBE_MIN_GAIN=1e-6 ENOMOTO_T_MIP_CUT_CLIQUE_EXTEND_MIN_FRAC=0.5"
ALL="physiciansched6-2 drayage-25-23 nursesched-sprint02 nw04 air03 noswot hypothyroid-k1 neos-960392 timtab1 supportcase26"
for r in 1 2 3 4; do
  if [ $((r % 2)) = 1 ]; then order="def dp"; else order="dp def"; fi
  for f in $order; do
    if [ $f = def ]; then e=""; else e=$DP; fi
    ENOMOTO_BENCH_ONLY="$ALL" analysis/heur_ls_20261023/bench_mix.sh dq$f $r $e
  done
done
echo BATCH DQ DONE
