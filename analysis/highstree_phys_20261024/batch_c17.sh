#!/bin/bash
# batch_c16.sh <回の始め> <回の終わり>: 9 問を 既定 (d17)・POOL + GUARD=fc (g17)・PROBE (pr17) で、計測と同じ呼び出し
# (scripts/bench_cand_20261020.py、60 秒・4 スレッド)。回ごとに形の順を入れ替える (1: d g pr、2: g pr d、3: pr d g、4: d g pr)
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_ONLY="noswot mc11 hypothyroid-k1 dcmulti neos-960392 roll3000 physiciansched6-2 h80x6320d binkar10_1"
export ENOMOTO_BENCH_SHA=$(git rev-parse --short=7 HEAD)
POOL="ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10 ENOMOTO_MIP_CUTSEL_HIGHS_GUARD=fc"
PROBE="ENOMOTO_MIP_CUTSEL_PROBE=rate ENOMOTO_T_MIP_CUTSEL_PROBE_ROUNDS=6"
run() { case $1 in d) python3 scripts/bench_cand_20261020.py d17 $2;; g) python3 scripts/bench_cand_20261020.py g17 $2 $POOL;; p) python3 scripts/bench_cand_20261020.py pr17 $2 $PROBE;; esac; }
for r in $(seq $1 $2); do
  case $((r % 2)) in 1) o="d p";; 0) o="p d";; esac
  for c in $o; do run $c $r; done
done
