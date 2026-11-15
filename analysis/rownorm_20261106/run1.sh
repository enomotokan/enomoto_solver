#!/bin/bash
# 単発: run1.sh <問題> <タグ> [環境変数...] (ENOMOTO、4 スレッド、時間は TL 秒 (既定 600)、ENOMOTO_MIP_LOG=1)
# 結果 (worker の JSON) は <タグ>_<問題>.out、ログは <タグ>_<問題>.log。走っている間はリポジトリの外 (W) に書き、終わってから移す
cd /home/user/enomoto_solver
P=$1; T=$2; shift 2
D=analysis/rownorm_20261106
W=/tmp/claude-0/-home-user-enomoto-solver/a5f7a634-6cc1-5fca-8eb4-42ca5e3d5078/scratchpad/runs; mkdir -p $W
env $(env | grep -o '^ENOMOTO_[A-Z0-9_]*' | sed 's/^/-u /') ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit ${TL:-600} --threads 4 > $W/${T}_$P.out 2> $W/${T}_$P.log
mv $W/${T}_$P.out $W/${T}_$P.log $D/
echo "$T $P $(tail -c 400 $D/${T}_$P.out)" >> $D/summary.txt
