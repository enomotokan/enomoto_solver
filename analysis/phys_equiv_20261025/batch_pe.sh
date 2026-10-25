#!/bin/bash
# 確かめの計測 (60 秒・4 スレッド、bench_mix.sh と同じ呼び出し)。候補 pecce = E2 + POOL + ENOMOTO_MIP_CUT_CLIQUE_EXTEND
cd "$(dirname "$0")/../.."
export ENOMOTO_BENCH_SHA=20bc1ec
CAND="ENOMOTO_MIP_PROBE_EQUIV2=1 ENOMOTO_MIP_ROOT_CUT_HIGHS=1 ENOMOTO_T_MIP_ROOT_CUT_HIGHS_MAX_CUTS=1000 ENOMOTO_MIP_CUTSEL_HIGHS=1 ENOMOTO_MIP_CUTSEL_HIGHS_POOL=1 ENOMOTO_T_MIP_ROOT_POOL_ROUNDS=1000 ENOMOTO_T_MIP_ROOT_CUT_AGE=10 ENOMOTO_MIP_CUT_CLIQUE_EXTEND=1"
SIDE="nursesched-sprint02 mzzv42z neos-960392 noswot hypothyroid-k1 nw04 10teams h80x6320d roll3000"
for r in 1 2 3 4; do ENOMOTO_BENCH_ONLY="physiciansched6-2" analysis/heur_ls_20261023/bench_mix.sh pecce $r $CAND; done
ENOMOTO_BENCH_ONLY="$SIDE" analysis/heur_ls_20261023/bench_mix.sh pedef 1
ENOMOTO_BENCH_ONLY="$SIDE" analysis/heur_ls_20261023/bench_mix.sh pecce 1 $CAND
ENOMOTO_BENCH_ONLY="$SIDE" analysis/heur_ls_20261023/bench_mix.sh pecce 2 $CAND
ENOMOTO_BENCH_ONLY="$SIDE" analysis/heur_ls_20261023/bench_mix.sh pedef 2
echo BATCH DONE
