#!/bin/bash
# 根だけ (ENOMOTO_MIP_NODE_LIMIT=1) で fallback の判定の値を集める: scan.sh <out.txt> <problems...>
d=/home/user/enomoto_solver/analysis/drayage_clique_runs
out=$1; shift
for p in "$@"; do
  l=$(cd /tmp && env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_NODE_LIMIT=1 ENOMOTO_MIP_CLIQUE_CUTS=fallback $EXTRA python $d/eno_run.py /home/user/enomoto_solver/.miplib_cache/$p.mps 60 2>&1 | grep -E "clique fallback|^Root cuts" | head -3 | tr '\n' ' ')
  echo "$p: $l" >> $out
done
