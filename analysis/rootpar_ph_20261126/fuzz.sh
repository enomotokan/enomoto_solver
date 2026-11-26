#!/bin/bash
# 試作の誤答の確かめ (analysis/unify_20261126/fuzz.sh の写し、fuzz_bigact を含む。ログの置き場所だけ変えた)
# リポジトリの根から: bash analysis/rootpar_ph_20261126/fuzz.sh <名前> <環境変数=値> ...  (例: rs ENOMOTO_MIP_NODE_ROWSCOPE=1)
# ログは analysis/rootpar_ph_20261126/logs_fuzz_<名前>/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/rootpar_ph_20261126/logs_fuzz_$1; mkdir -p $L
for kv in "${@:2}"; do export "$kv"; done
export PATH=${FUZZ_VENV:-$PWD/.venv}/bin:$PATH
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run fuzz_bigact env ENOMOTO_MIP_REL_GAP=1e-9 python3 analysis/unify_20261126/fuzz_bigact.py 300 0
run fuzz_bigact2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 analysis/unify_20261126/fuzz_bigact.py 300 5000
export CARGO_TARGET_DIR=target/libtest RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (${2:-既定})"; cargo test --release --lib > $L/cargo.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo.log
