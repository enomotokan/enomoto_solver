#!/bin/bash
# 既定にした ENOMOTO_MIP_PROOFCOST=pa (環境変数なし) の誤答の確かめ。analysis/release0110_20261120/fuzz.sh と同じ 7 本と、LP の fuzz 3 本・mip_fuzz_symmetric。
# 環境変数はつけない。リポジトリの根から: bash analysis/proofcost_20261123/adopt/fuzz.sh
# ログは analysis/proofcost_20261123/adopt/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/proofcost_20261123/adopt/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
# LP の確かめ (proofcost は MIP の中だけだが、今までの既定化と同じ手順で回す)
run lp_fuzz_ipmxo python3 scripts/lp_fuzz_vs_highs.py 300 ipm_crossover
run lp_fuzz_auto python3 scripts/lp_fuzz_vs_highs.py 300 auto
run lp_fuzz_simplex python3 scripts/lp_fuzz_vs_highs.py 300 simplex
run mip_fuzz_sym python3 scripts/mip_fuzz_symmetric.py 300 0 2
echo "== 終わり (LP も)"
