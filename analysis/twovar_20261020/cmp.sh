#!/bin/bash
# cmp.sh <出力ディレクトリ> <秒> <回数> <問題 ...>: 設定ごと (CONFIGS) に ENOMOTO を回してログを残す
# 設定: base (既定)、all4 (前の調査の試作 4 つ)、pc (all4 + PROBE_CLIQUE)、full (pc + SPARSIFY + DOMCOL)、
#       pc1 (PROBE_CLIQUE だけ)、sd (SPARSIFY + DOMCOL だけ)
out=$1; t=$2; k=$3; shift 3
D=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$D/../.." && pwd)
mkdir -p "$out"
ALL4="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_INEQ_SINGLETON=1"
declare -A CF
CF[base]="ENOMOTO_X=1"
CF[all4]="$ALL4"
CF[pc]="$ALL4 ENOMOTO_MIP_PROBE_CLIQUE=1"
CF[full]="$ALL4 ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_SPARSIFY=1 ENOMOTO_MIP_DOMCOL=1"
CF[pc1]="ENOMOTO_MIP_PROBE_CLIQUE=1"
CF[sd]="ENOMOTO_MIP_SPARSIFY=1 ENOMOTO_MIP_DOMCOL=1"
CF[pcsd]="ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_SPARSIFY=1 ENOMOTO_MIP_DOMCOL=1"
for p in "$@"; do
  for c in ${CONFIGS:-base pc full}; do
    for r in $(seq 1 $k); do
      env ${CF[$c]} ENOMOTO_MIP_LOG=1 timeout $((t+120)) python -I $D/eno_run.py $R/.miplib_cache/$p.mps $t > $out/${p}_${c}_$r.log 2>&1
      echo "$p $c $r $(grep '^Presolve:' $out/${p}_${c}_$r.log | head -1 | sed 's/.*-> //') | $(grep RESULT $out/${p}_${c}_$r.log | cut -c1-260)"
    done
  done
done
