#!/bin/bash
# 追加の依頼の計測 (60 秒・4 スレッド、bench_mix.sh と同じ呼び出し): batch_pq.sh <1|2>。1 は 既定→A→B→C、2 は C→B→A→既定 の順。
# 10 問は 1 回ずつ、physiciansched6-2 は各形 2 回ずつ (回の番号 1・3 / 2・4)
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_SHA=cf69878
A="ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1"
B="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1"
C="$B ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6 ENOMOTO_T_MIP_CUTSEL_PROBE_RATIO=3"
ALL="physiciansched6-2 noswot hypothyroid-k1 h80x6320d roll3000 10teams nw04 nursesched-sprint02 mzzv42z neos-960392"
run() { ENOMOTO_BENCH_ONLY="$2" analysis/heur_ls_20261023/bench_mix.sh $1 $3 ${@:4}; }
if [ "$1" = 1 ]; then order="def A B C"; r=1; r2=3; else order="C B A def"; r=2; r2=4; fi
for f in $order; do
  case $f in def) e="";; A) e=$A;; B) e=$B;; C) e=$C;; esac
  run pq$f "$ALL" $r $e
  run pq$f physiciansched6-2 $r2 $e
done
echo BATCH $1 DONE
