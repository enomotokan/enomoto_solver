#!/bin/bash
# 40 問 + 16 問 (4 スレッド・60 秒): bench_mix.sh <名前> <回> [ENV=VAL ...]。scripts/bench_cand_20261020.py で、結果は benchmarks/ に
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_SHA=${ENOMOTO_BENCH_SHA:-$(git rev-parse --short=7 HEAD)}
python3 scripts/bench_cand_20261020.py "$@"
