#!/bin/bash
# 基準と試作を比べる: cmp.sh <問題> <秒> <回数> ; ログは cmp/<問題>_<設定>_<回>.log
D=analysis/physiciansched6-2_20261020
p=$1; T=$2; R=$3
declare -A CFG
CFG[base]=""
CFG[ce]="ENOMOTO_MIP_CLIQUE_EXTEND=1"
CFG[all]="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_INEQ_SINGLETON=1"
for r in $(seq 1 $R); do for c in ${CFGS:-base ce all}; do
  env ${CFG[$c]} ENOMOTO_MIP_LOG=1 timeout $((T+200)) python $D/eno_run.py .miplib_cache/$p.mps $T > $D/cmp/${p}_${c}_$r.log 2>&1
  python $D/summ.py $D/cmp/${p}_${c}_$r.log "$p $c #$r"
done; done
