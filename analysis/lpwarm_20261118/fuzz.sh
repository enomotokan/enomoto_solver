#!/bin/bash
# 試作 ENOMOTO_SIMPLEX_BFRT_GROUP の誤答の確かめ: release019 (analysis/release019_20261117/fuzz.sh) と同じ fuzz 7 本を、試作の値 ($1、既定 gz) で回す。
# リポジトリの根から: bash analysis/lpwarm_20261118/fuzz.sh [値]   ログは analysis/lpwarm_20261118/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
V=${1:-gz}
PY=${PY:-.venv/bin/python}
L=analysis/lpwarm_20261118/fuzz_logs/$V; mkdir -p $L
export ENOMOTO_SIMPLEX_BFRT_GROUP=$V
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 $PY scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint $PY analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 $PY analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
