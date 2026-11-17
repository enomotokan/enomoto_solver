#!/bin/bash
# 伝播の締めすぎの直し (ENOMOTO_T_PROP_RELTOL=1e-10、既定は 0 のまま) の誤答の確かめ。release019 の 7 本と、sat_rens の形の乱数問題
# (analysis/dualfree_20261117/fuzz.py、種 0〜19999) を既定 (試作なし) と pcd で回す。リポジトリの根から: bash analysis/proptol_20261117/fuzz.sh [値] [環境変数の名前 (既定 ENOMOTO_T_PROP_RELTOL)]
set -u
cd "$(dirname "$0")/../.."
PY=${PY:-.venv/bin/python}
V=${1:-1e-10}
K=${2:-ENOMOTO_T_PROP_RELTOL}
L=analysis/proptol_20261117/fuzz_logs/$K; mkdir -p $L
export $K=$V
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "^bad|^seed|mismatch|不一致" $L/$name.log | tail -30; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 $PY scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint $PY analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 $PY analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
for a in 0 10000; do
  run dfi_def_$a $PY analysis/dualfree_20261117/fuzz.py 10000 $a off
  run dfi_pcd_$a env ENOMOTO_AGG_PIVOT_COL=1 $PY analysis/dualfree_20261117/fuzz.py 10000 $a
done
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test"; $CT > $L/cargo.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo.log
