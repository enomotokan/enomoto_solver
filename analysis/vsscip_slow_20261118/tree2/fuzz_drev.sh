#!/bin/bash
# 群 2 の試作 drev (ENOMOTO_MIP_DOMCOL + _PRED + _REV) の誤答の確かめ。release019_20261117/fuzz.sh の組を環境変数つきで回す。
# リポジトリの根から: bash analysis/vsscip_slow_20261118/tree2/fuzz_drev.sh
set -u
cd "$(dirname "$0")/../../.."
export ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DOMCOL_PRED=1 ENOMOTO_MIP_DOMCOL_REV=1
L=analysis/vsscip_slow_20261118/tree2/logs/fuzz; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
