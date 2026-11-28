#!/bin/bash
# 0.2.1 (0.2.0 + 幅の小さい列の固定の誤答の直し 9e11b2c4c だけ) の誤答の確かめ。0.2.0 と同じ組 + fuzz_domcol。
# リポジトリの根から: bash analysis/release021_20261128/fuzz.sh  (PYTHONPATH でこの木の python/ を使う)
set -u
cd "$(dirname "$0")/../.."
export PYTHONPATH=$PWD/python
L=analysis/release021_20261128/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -2 $L/$name.log; }
run check_default python3 analysis/protoscreen_20261124/check_default.py
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run fuzz_domcol python3 analysis/pregd_20261128/fuzz_domcol.py 10000 0
