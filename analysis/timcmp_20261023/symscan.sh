#!/bin/bash
# symscan.sh: 55 問で、対称性の検出の結果を既定と ENOMOTO_MIP_SYM_SKIP_FIXED=1 で比べる (各 2 秒、1 スレッド)
cd "$(dirname "$0")"
for m in ../../.miplib_cache/*.mps; do
  p=$(basename $m .mps)
  a=$(ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 timeout 60 python3 ../heur_ls_20261023/eno_run.py $m 2 1 2>&1 | grep -E "MIP: symmetry: [0-9]+ generators|MIP: symmetry: none|full orbitopes" | sed 's/MIP: symmetry: //' | tr '\n' ' ')
  b=$(ENOMOTO_MIP_SYM_SKIP_FIXED=1 ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_LOG=1 timeout 60 python3 ../heur_ls_20261023/eno_run.py $m 2 1 2>&1 | grep -E "MIP: symmetry: [0-9]+ generators|MIP: symmetry: none|full orbitopes" | sed 's/MIP: symmetry: //' | tr '\n' ' ')
  echo "$p | $a | $b"
done
