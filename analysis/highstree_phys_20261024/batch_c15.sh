#!/bin/bash
# batch_c15.sh <名前> [ENV=VAL ...]: 計測 14 で得・損の出た 9 問を、計測と同じ呼び出し (scripts/bench_cand_20261020.py、60 秒・4 スレッド) で 2 回
cd "$(dirname "$0")/../.."
name=$1; shift
export ENOMOTO_BENCH_ONLY="noswot mc11 hypothyroid-k1 dcmulti neos-960392 roll3000 physiciansched6-2 h80x6320d binkar10_1"
export ENOMOTO_BENCH_SHA=$(git rev-parse --short=7 HEAD)
for r in 1 2; do python3 scripts/bench_cand_20261020.py $name $r "$@"; done
