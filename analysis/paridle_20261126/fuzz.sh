#!/bin/bash
# 試作の誤答の確かめ (analysis/globalfix_20261124/fuzz.sh の写しで、環境変数を何個でも渡せるようにした): 同じ 7 本と cargo test を、引数の環境変数をつけて回す。
# リポジトリの根から: bash analysis/paridle_20261126/fuzz.sh <名前> <環境変数=値> ...  (例: eh4 ENOMOTO_MIP_PARIDLE=eh ENOMOTO_MIP_THREADS=4)
# ログは analysis/paridle_20261126/logs/fuzz_<名前>/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/paridle_20261126/logs/fuzz_$1; mkdir -p $L
NM=$1; shift; for kv in "$@"; do export "$kv"; done
export PATH=$PWD/.venv/bin:$PATH
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -3 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
export CARGO_TARGET_DIR=target/libtest RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test ($NM)"; cargo test --release --lib > $L/cargo.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo.log
