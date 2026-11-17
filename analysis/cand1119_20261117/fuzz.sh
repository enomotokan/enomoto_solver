#!/bin/bash
# cand1119 の誤答の確かめ。analysis/cand1118_default_20261116/fuzz.sh と同じ組を、cand1119 の環境変数 (cfg.py の CAND1119) を付けて回す。
# cargo test は既定のまま (環境変数なし)。リポジトリの根から: bash analysis/cand1119_20261117/fuzz.sh
# ログは analysis/cand1119_20261117/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/cand1119_20261117/fuzz_logs; mkdir -p $L
C="ENOMOTO_MIP_CLIQUE_CUTS=sigmoid ENOMOTO_T_MIP_CLIQUE_SIG_A=1.8 ENOMOTO_T_MIP_CLIQUE_SIG_B=0.41 ENOMOTO_MIP_SYM_TWINS=1 ENOMOTO_MIP_CUTSEL_PROBE_AFFORD=1"
run() { name=$1; shift; echo "== $name: $*"; env $C "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
# 4 スレッドの揺れで出る誤りも拾うため、種を変えてもう 1 回ずつ
run fuzz_tiny2 ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
# 双子の併合 (SYM_TWINS) が働く形: 同じ整数列を含む乱数問題 (analysis/parint_20261028/fuzz_dupcols.py)
[ -f analysis/parint_20261028/fuzz_dupcols.py ] && run fuzz_dupcols python3 analysis/parint_20261028/fuzz_dupcols.py 1000
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
