#!/bin/bash
# 診断: ENOMOTO_MIP_MODKVB_DUMP つきの根 (ノード上限 1) を /tmp/pyw3 の wheel (DUMP の入ったもの) で。使い方: ./dump.sh 問題 [時間]
p=$1; tl=${2:-60}; cd "$(dirname "$0")"; WT=$(cd ../..; pwd)
mps=$(cd $WT && python -c "import sys; sys.path.insert(0,'scripts'); import run_miplib_benchmark as rb; from pathlib import Path; print(rb.fetch('$p', Path('.miplib_cache')))")
mkdir -p dump
(cd $WT && env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') PYTHONPATH=/tmp/pyw3 ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_T_MIP_SEED=1 ENOMOTO_MIP_NODE_LIMIT=1 ENOMOTO_MIP_MODKVB=1 ENOMOTO_MIP_MODKVB_LOG=1 ENOMOTO_MIP_MODKVB_DUMP=1 \
  python scripts/run_miplib_benchmark.py --worker-ours $mps --time-limit $tl --threads 4 2>&1 >/dev/null | gzip > analysis/modkvb_20261107/dump/$p.err.gz)
