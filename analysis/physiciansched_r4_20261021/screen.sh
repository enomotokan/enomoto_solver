#!/bin/bash
# screen.sh: 40 + 16 問で、同値の代入の試作が前処理後の形を変えるかを 3 秒で見る (logs/screen.txt)
cd "$(dirname "$0")"
PS=$(python3 -c "import sys,json; sys.path.insert(0,'../../scripts'); import run_miplib_benchmark as r; d=json.load(open('../../benchmarks/highsonly16_4t_60s_20261020_bc89ad1_highsgap0.json')); print(' '.join(r.PROBLEMS+[k for k in d if k not in r.PROBLEMS]))")
one() { env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" timeout 90 python3 eno_run.py ../../.miplib_cache/$p.mps 3 2>&1 | grep -E "equivalent binary pairs|equivalence pass|^Presolve: [0-9]+ rows, [0-9]+ columns -> " | head -8 | sed 's/MIP: probing: //; s/equivalent binary pairs/eq/; s/equivalence pass with implications: /eq2 /; s/ pairs found/p/; s/Presolve: //; s/[0-9]* rows, [0-9]* columns -> //' | tr '\n' ' '; }
for p in $PS; do
  echo "$p | def: $(one) | ef: $(one ENOMOTO_MIP_PROBE_EQUIV_FIX=1) | efc: $(one ENOMOTO_MIP_PROBE_EQUIV_FIX=1 ENOMOTO_MIP_PROBE_CLIQUE=equiv) | pcr: $(one ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_PROBE_REPRESOLVE=1)"
done
