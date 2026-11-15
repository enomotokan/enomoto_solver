#!/bin/bash
# D2 の誤答の確かめ。cand1114 の fuzz.sh と同じ組を CAND1114 + ENOMOTO_MIP_HL2_D2=ab で。リポジトリの根から: bash analysis/d2_20261113/fuzz.sh
# cfg.py の d2ab を全部置いて回す。cargo test は既定と d2ab の 2 回。ログは analysis/d2_20261113/fuzz_logs/ (コミットしない)。
set -u
cd "$(dirname "$0")/../.."
L=analysis/d2_20261113/fuzz_logs; mkdir -p $L
ENVS=$(python3 -c "import sys; sys.path.insert(0, 'analysis/d2_20261113'); from cfg import CFGS; print(' '.join(f'{k}={v}' for k, v in CFGS['d2ab'].items()))")
run() { name=$1; shift; echo "== $name: $*"; env $ENVS "$@" > $L/$name.log 2>&1; echo "rc=$?"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
echo "== cargo test (d2ab)"; env $ENVS $CT > $L/cargo_cand.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_cand.log
