#!/bin/bash
# run_bench_worker.sh <名前> [環境変数=値 ...] -> logs/<名前>.log
# bench_mix.sh (scripts/bench_cand_20261020.py → run_miplib_benchmark.py --worker-ours) と同じ呼び出し (4 スレッド・60 秒、
# ENOMOTO_MIP_THREADS=4、log 引数なし)。ENOMOTO_MIP_LOG は環境変数で足す
cd "$(dirname "$0")/../.."
name=$1; shift
env ENOMOTO_MIP_THREADS=4 "$@" python3 scripts/run_miplib_benchmark.py --worker-ours .miplib_cache/neos-960392.mps --time-limit 60 --threads 4 > analysis/highstree_n960_20261024/logs/$name.log 2>&1
echo "$name $(tail -1 analysis/highstree_n960_20261024/logs/$name.log)"
