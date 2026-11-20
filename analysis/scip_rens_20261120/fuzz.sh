#!/bin/bash
# LPFACE の試作の誤答の確かめ (release019 の fuzz 7 本の写し)。環境変数は呼ぶ側で export し、TAG でログの置き場を分ける。
# リポジトリの根から: ENOMOTO_MIP_LPFACE=cf TAG=cf bash analysis/scip_rens_20261120/fuzz.sh
# ログは analysis/scip_rens_20261120/fuzz_logs/<TAG>/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/scip_rens_20261120/fuzz_logs/${TAG:-x}; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
# 4 スレッドの揺れで出る誤りも拾うため、種を変えてもう 1 回ずつ
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
