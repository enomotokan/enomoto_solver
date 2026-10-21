#!/bin/bash
# screen2.sh: 40 + 16 問で ENOMOTO_MIP_PROBE_EQUIV2 が働くか (2 巡目の組の数・時間・前処理後の大きさ) を 3 秒で見る (logs/screen2.txt)
cd "$(dirname "$0")"
PS=$(python3 -c "import sys,json; sys.path.insert(0,'../../scripts'); import run_miplib_benchmark as r; d=json.load(open('../../benchmarks/highsonly16_4t_60s_20261020_bc89ad1_highsgap0.json')); print(' '.join(r.PROBLEMS+[k for k in d if k not in r.PROBLEMS]))")
one() { env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 "$@" timeout 90 python3 eno_run.py ../../.miplib_cache/$p.mps 3 2>&1 | grep -E "equivalent binary pairs|equivalence pass|^Presolve: [0-9]+ rows, [0-9]+ columns -> " | head -8 | sed 's/MIP: probing: //; s/equivalent binary pairs/eq/; s/equivalence pass with implications: /eq2 /; s/ pairs found/p/; s/Presolve: //; s/[0-9]* rows, [0-9]* columns -> //' | tr '\n' ' '; }
for p in $PS; do
  echo "$p | def: $(one) | e2: $(one ENOMOTO_MIP_PROBE_EQUIV2=1)"
done
