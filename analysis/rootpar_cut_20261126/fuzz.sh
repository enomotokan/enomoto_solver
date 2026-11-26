#!/bin/bash
# ROOTPAR_CUT の既定化 (2026-11-26) の誤答の確かめ。release0110 の fuzz.sh と同じ組を、環境変数なし (既定) で回す。
# リポジトリの根から: bash analysis/rootpar_cut_20261126/fuzz.sh
# ログは analysis/rootpar_cut_20261126/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/rootpar_cut_20261126/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name: $*"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
# 4 スレッドの揺れで出る誤りも拾うため、種を変えてもう 1 回ずつ
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
# 4 スレッド (この直しは 2 スレッド以上の根の分離でだけ働く)
for s in 0 7000; do
  run objint_t4_$s env ENOMOTO_MIP_THREADS=4 python3 analysis/objint_20261112/fuzz_objint.py 1000 $s
  run slack_t4_$s env ENOMOTO_MIP_THREADS=4 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 $s
done
