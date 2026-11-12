#!/bin/bash
# PRESOLVE2: 58 問で、根の LP の前の大きさを既定と試作で並べる (epre.py、結果は epre.jsonl)。前処理が既定と変わる問題を探すため。
cd "$(dirname "$0")"
P=$(python3 -c "import sys; sys.path.insert(0,'../cbcimpl_20261029'); from probs import PROBS; print(' '.join(PROBS))")
for p in $P; do
  python3 epre.py def $p
  python3 epre.py spl $p ENOMOTO_MIP_SPARSIFY_LATE=1
  python3 epre.py enum $p ENOMOTO_MIP_HO3_ENUM=1
  python3 epre.py deq $p ENOMOTO_MIP_DUALEQ=1
done
