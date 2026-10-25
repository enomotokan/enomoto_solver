#!/bin/bash
# 追加の依頼の計測 2: D = C + ENOMOTO_MIP_CUTSEL_PROBE_ALT=NS、CK = C + ENOMOTO_MIP_CUTSEL_PROBE_KEEP=1。既定・C は batch_pq.sh の pqdef・pqC を使う
# 1 巡目 D→CK、2 巡目 CK→D。10 問は 1 回ずつ、physiciansched6-2 は各形 2 回ずつ
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_SHA=22d8dea
C="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1 ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3"
ALL="physiciansched6-2 noswot hypothyroid-k1 h80x6320d roll3000 10teams nw04 nursesched-sprint02 mzzv42z neos-960392"
run() { ENOMOTO_BENCH_ONLY="$2" analysis/heur_ls_20261023/bench_mix.sh $1 $3 ${@:4}; }
for pass in 1 2; do
  if [ $pass = 1 ]; then order="D CK"; r=1; r2=3; else order="CK D"; r=2; r2=4; fi
  for f in $order; do
    case $f in D) e="$C ENOMOTO_MIP_CUTSEL_PROBE_ALT=NS";; CK) e="$C ENOMOTO_MIP_CUTSEL_PROBE_KEEP=1";; esac
    run pq$f "$ALL" $r $e
    run pq$f physiciansched6-2 $r2 $e
  done
done
echo BATCH DK DONE
