#!/bin/bash
# pcd (ENOMOTO_AGG_PIVOT_COL=1 + ENOMOTO_AGG_DUALFREE_INEQ=1) の誤答・数値の害の確かめ。release019 の fuzz.sh と同じ 7 本を pcd の環境変数で回し、
# sat_rens の形の乱数問題 (analysis/dualfree_20261117/fuzz.py: 大きい M の行 (係数 1e3〜2e7)・境界が導ける値ちょうどと ±1e-10〜1e-6 の許容の境目)
# を 2 万問足す (pc だけの 1 万問も)。リポジトリの根から: bash analysis/pcd_20261117/fuzz.sh
# ログは analysis/pcd_20261117/fuzz_logs/ (コミットしない)。python は highspy の入った venv のものを PY で渡す (既定 .venv/bin/python)。
set -u
cd "$(dirname "$0")/../.."
PY=${PY:-.venv/bin/python}
L=analysis/pcd_20261117/fuzz_logs; mkdir -p $L
export ENOMOTO_AGG_PIVOT_COL=1 ENOMOTO_AGG_DUALFREE_INEQ=1
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "^bad|^seed|mismatch|wrong|不一致" $L/$name.log | tail -8; tail -2 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 $PY scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint $PY analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 $PY benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 $PY analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 $PY analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
# sat_rens の形 (fuzz.py は ENOMOTO_AGG_DUALFREE_INEQ を自分で付ける。off を付けると外す)
run fuzz_dfi_pcd $PY analysis/dualfree_20261117/fuzz.py 20000 0
run fuzz_dfi_pc env -u ENOMOTO_AGG_DUALFREE_INEQ $PY analysis/dualfree_20261117/fuzz.py 10000 0 off
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; env -u ENOMOTO_AGG_PIVOT_COL -u ENOMOTO_AGG_DUALFREE_INEQ $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
