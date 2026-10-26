#!/bin/bash
# screen.sh: 40 問で、同値の代入 (PROBE_CLIQUE + PROBE_REPRESOLVE) が働くか (同値な組の数・前処理後の大きさ) を 3 秒で見る
cd "$(dirname "$0")"
P5S="ENOMOTO_MIP_INEQ_SINGLETON=1 ENOMOTO_MIP_DUAL_UB=1 ENOMOTO_MIP_CS_OBJ_PIVOT=1 ENOMOTO_MIP_FJ_LP_REPAIR=1 ENOMOTO_MIP_INC_LP_CAP=1"
for p in $(python3 -c "import sys; sys.path.insert(0,'../../scripts'); import run_miplib_benchmark as r; print(' '.join(r.PROBLEMS))"); do
  a=$(env $P5S ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 timeout 60 python3 eno_run.py ../../.miplib_cache/$p.mps 3 2>&1 | grep -E "equivalent binary pairs|^Presolve: [0-9]+ rows, [0-9]+ columns -> " | head -3 | sed 's/MIP: probing: //; s/equivalent binary pairs/eq/; s/Presolve: //' | tr '\n' ' ')
  b=$(env $P5S ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_PROBE_REPRESOLVE=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 timeout 60 python3 eno_run.py ../../.miplib_cache/$p.mps 3 2>&1 | grep -E "equivalent binary pairs|^Presolve: [0-9]+ rows, [0-9]+ columns -> " | sed 's/MIP: probing: //; s/equivalent binary pairs/eq/; s/Presolve: //' | tr '\n' ' ')
  echo "$p | p5s: $a | pcr: $b"
done
