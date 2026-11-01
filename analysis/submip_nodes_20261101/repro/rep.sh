#!/bin/bash
# h80x6320d を ns250s で、6386 を最適と返す回が出るまで (最大 20 回) 回し、再スタートの模型を書き出す
cd /home/user/enomoto_solver
for i in $(seq 1 20); do
  rm -f /tmp/rdump/h80_r*.mps
  env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP=250 ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL=1 ENOMOTO_MIP_RENS_MORE_NODES_REAL=1 ENOMOTO_MIP_DEBUG_DUMP_RESTART=/tmp/rdump/h80 \
    python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/h80x6320d.mps --time-limit 60 --threads 4 > /tmp/rdump/out_$i.json 2> /tmp/rdump/log_$i.txt
  o=$(grep -o '"objective": [0-9.]*' /tmp/rdump/out_$i.json)
  echo "$i $o"
  if echo "$o" | grep -q 6386; then mkdir -p /tmp/rdump/hit; cp /tmp/rdump/h80_r*.mps /tmp/rdump/log_$i.txt /tmp/rdump/hit/; echo HIT; break; fi
done
