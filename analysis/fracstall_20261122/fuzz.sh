#!/bin/bash
# 試作 ENOMOTO_MIP_CUT_STALL_FRAC=s の誤答の確かめ。analysis/release0110_20261120/fuzz.sh と同じ 7 本。
# リポジトリの根から: bash analysis/fracstall_20261122/fuzz.sh
# ログは analysis/fracstall_20261122/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/fracstall_20261122/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_CUT_STALL_FRAC="${SF:-s}"
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
echo "== 終わり"
