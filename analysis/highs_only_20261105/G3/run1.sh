#!/bin/bash
# 単発: run1.sh <solver: ours|highs> <問題> <タグ> [環境変数...]
# 結果 (worker の JSON) は <タグ>_<問題>.out、ログは <タグ>_<問題>.log。走っている間はリポジトリの外 (W) に書き、終わってから移す
# (コミットの rebase が書き込み中のファイルを作り直して結果が失われたため)
cd /home/user/enomoto_solver
S=$1; P=$2; T=$3; shift 3
D=analysis/highs_only_20261105/G3
W=/tmp/claude-0/-home-user-enomoto-solver/9de22911-6f4d-530c-ae3b-e7d47498b968/scratchpad/runs; mkdir -p $W
if [ "$S" = highs ]; then
  env ENOMOTO_BENCH_HIGHS_LOG=1 "$@" python scripts/run_miplib_benchmark.py --worker-highs .miplib_cache/$P.mps --time-limit ${TL:-600} --threads 4 > $W/${T}_$P.out 2> $W/${T}_$P.log
else
  env ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_THREADS=4 "$@" python scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/$P.mps --time-limit ${TL:-600} --threads 4 > $W/${T}_$P.out 2> $W/${T}_$P.log
fi
mv $W/${T}_$P.out $W/${T}_$P.log $D/
echo "$T $P $(tail -c 400 $D/${T}_$P.out)" >> $D/summary.txt
