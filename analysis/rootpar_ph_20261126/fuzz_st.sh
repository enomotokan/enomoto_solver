#!/bin/bash
# 最後の研磨の確かめ (f/g) で not_solved になった fuzz 4 本だけを回す (README 5.7 節)。bash fuzz_st.sh <名前> <環境変数=値> ...
set -u
cd "$(dirname "$0")/../.."
L=analysis/rootpar_ph_20261126/logs_fuzz_$1; mkdir -p $L
for kv in "${@:2}"; do export "$kv"; done
export PATH=$PWD/.venv/bin:$PATH
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -h "^seed\|^bad\|^tol" $L/$name.log | tail -12; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
