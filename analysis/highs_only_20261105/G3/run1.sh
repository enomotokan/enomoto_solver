#!/bin/bash
# 単発: run1.sh <solver: ours|highs> <問題> <タグ> [環境変数...]
# 結果 (worker の JSON) は <タグ>_<問題>.out、ログは <タグ>_<問題>.log (後で gzip)
cd /home/user/enomoto_solver
S=$1; P=$2; T=$3; shift 3
D=analysis/highs_only_20261105/G3
if [ "$S" = highs ]; then
  env ENOMOTO_BENCH_HIGHS_LOG=1 "$@" python scripts/run_miplib_benchmark.py --worker-highs .miplib_cache/$P.mps --time-limit ${TL:-600} --threads 4 > $D/${T}_$P.out 2> $D/${T}_$P.log
else
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit ${TL:-600} --threads 4 > $D/${T}_$P.out 2> $D/${T}_$P.log
fi
echo "$T $P $(tail -c 400 $D/${T}_$P.out)" >> $D/summary.txt
