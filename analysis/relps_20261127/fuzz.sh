#!/bin/bash
# 試作 ENOMOTO_MIP_RELPS の誤答の確かめ (analysis/comboharm_20261124/fuzz.sh の写し): 同じ 7 本と cargo test を、引数の環境変数をつけて回す。
# リポジトリの根から: bash analysis/relps_20261127/fuzz.sh <名前> <環境変数=値> ...  ログは analysis/relps_20261127/logs_fuzz_<名前>/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/relps_20261127/logs_fuzz_$1; mkdir -p $L
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
export CARGO_TARGET_DIR=target/libtest RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test ($2)"; cargo test --release --lib > $L/cargo.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo.log
