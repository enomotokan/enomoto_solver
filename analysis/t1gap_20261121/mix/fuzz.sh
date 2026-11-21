#!/bin/bash
# T1MIX (ENOMOTO_MIP_T1MIX=1) と T1RENS (ENOMOTO_MIP_T1RENS=1) の誤答の確かめ。analysis/rootgap_20261121/fuzz.sh と同じ 7 本を
# 1 スレッド (ENOMOTO_MIP_THREADS=1) で両方つけて回し、cargo test を既定で回す。リポジトリの根から: bash analysis/t1gap_20261121/mix/fuzz.sh
# ログは analysis/t1gap_20261121/mix/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../../.."
L=analysis/t1gap_20261121/mix/fuzz_logs; mkdir -p $L
export ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_T1MIX=1 ENOMOTO_MIP_T1RENS=1
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
unset ENOMOTO_MIP_THREADS ENOMOTO_MIP_T1MIX ENOMOTO_MIP_T1RENS
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
