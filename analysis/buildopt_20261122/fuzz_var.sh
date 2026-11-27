#!/bin/bash
# 作り方を変えた版 (var/NAME) で誤答の確かめ。analysis/gainstall_20261122/fuzz_w.sh と同じ 7 本 (環境変数は既定のまま)。
# リポジトリの根から: bash analysis/buildopt_20261122/fuzz_var.sh NAME。ログは analysis/buildopt_20261122/fuzz_logs/NAME/。
set -u
V=$1
cd "$(dirname "$0")/../.."
B=$PWD/analysis/buildopt_20261122
export PYTHONPATH=$B/var/$V
python3 -c "import enomoto_solver._core as c, sys; assert c.__file__.startswith('$B/var/$V/'), c.__file__; print('core', c.__file__)" || exit 1
L=$B/fuzz_logs/$V; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
echo FUZZ_DONE $V
