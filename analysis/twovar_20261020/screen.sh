#!/bin/bash
# screen.sh <秒>: 40 問それぞれで、試作 (full) が何をするかを数える (同値な組・疎化した行・支配で固定した列・前処理後の大きさ)
t=$1
D=$(cd "$(dirname "$0")" && pwd)
R=$(cd "$D/../.." && pwd)
P=$(python3 -c "import sys; sys.path.insert(0,'$R/scripts'); import run_miplib_benchmark as r; print(' '.join(r.PROBLEMS))")
for p in $P; do
  for c in all4 full; do
    if [ $c = all4 ]; then E="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_INEQ_SINGLETON=1"; else E="ENOMOTO_MIP_CLIQUE_EXTEND=1 ENOMOTO_MIP_MERGE_REPRESOLVE=1 ENOMOTO_MIP_PROBE_EQUIV=1 ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_SPARSIFY=1 ENOMOTO_MIP_DOMCOL=1"; fi
    o=$(env $E ENOMOTO_MIP_LOG=1 timeout $((t+60)) python -I $D/eno_run.py $R/.miplib_cache/$p.mps $t 2>&1)
    eq=$(echo "$o" | grep "equivalent binary pairs" | awk '{s+=$3} END{print s+0}')
    sp=$(echo "$o" | grep "sparsified" | awk '{s+=$4} END{print s+0}')
    dc=$(echo "$o" | grep "dominated binaries fixed" | awk '{s+=$3} END{print s+0}')
    pre=$(echo "$o" | grep "^Presolve:" | head -1 | sed 's/.*-> //; s/ (.*//')
    echo "$p $c equiv=$eq sparsified=$sp domfix=$dc pre=[$pre]"
  done
done
