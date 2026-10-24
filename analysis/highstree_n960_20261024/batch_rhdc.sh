#!/bin/bash
# batch_rhdc.sh: ENOMOTO_MIP_RENS_HIGHS=dive + ENOMOTO_MIP_CUT_LP_CONTINUE=1 を 7 問 × 2 回 (bench と同じ呼び出し)。logs/batch_rhdc.txt
cd "$(dirname "$0")/../.."
L=analysis/highstree_n960_20261024/logs
for p in physiciansched6-2 mzzv42z timtab1 30n20b8 app1-1 qiu h80x6320d; do
  for i in 1 2; do
    env ENOMOTO_MIP_THREADS=4 ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_RENS_HIGHS=dive ENOMOTO_MIP_CUT_LP_CONTINUE=1 python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$p.mps --time-limit 60 --threads 4 > $L/b_${p}_rhdc$i.log 2>&1
    echo "$p rhdc$i $(tail -1 $L/b_${p}_rhdc$i.log)" >> $L/batch_rhdc.txt
  done
done
echo DONE >> $L/batch_rhdc.txt
