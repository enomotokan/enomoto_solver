#!/bin/bash
# 既定にした OVERRUN3=ild と ipmxm (XO_MIP_CLEANUP_HINT・XO_IPM_RETRY_PC・XO_NO_BOUND_STOP_ZERO_COST・IPM_KKT_AUG_RATIO、環境変数なし) の誤答の確かめ。analysis/release0110_20261120/fuzz.sh と同じ 7 本。
# 環境変数はつけない。リポジトリの根から: bash analysis/shelved_20261122/adopt2/fuzz.sh
# ログは analysis/shelved_20261122/adopt2/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/shelved_20261122/adopt2/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
# LP の確かめ (KKT_AUG_RATIO・RETRY_PC・NO_BOUND_STOP は MIP の外の内点法・クロスオーバーにも効く)
run lp_fuzz_ipmxo python3 scripts/lp_fuzz_vs_highs.py 300 ipm_crossover
run lp_fuzz_auto python3 scripts/lp_fuzz_vs_highs.py 300 auto
run lp_fuzz_simplex python3 scripts/lp_fuzz_vs_highs.py 300 simplex
run mip_fuzz_sym python3 scripts/mip_fuzz_symmetric.py 300 0 2
echo "== 終わり (LP も)"
