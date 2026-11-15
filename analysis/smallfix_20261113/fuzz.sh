#!/bin/bash
# SMALLFIX の誤答の確かめ。cand1114 の fuzz.sh と同じ組を、CAND1114 + C3 + D1 (cfg.py の c3d1) で回す。
# 加えて C3 の上限で止まった試しの LP の後始末を通すため、上限を 1 反復・0 秒にした組 (stress) でも fuzz_tiny・fuzz_highs を回す。
# リポジトリの根から: bash analysis/smallfix_20261113/fuzz.sh   ログは analysis/smallfix_20261113/fuzz_logs/ (gzip にしてコミット)。
set -u
cd "$(dirname "$0")/../.."
CFG=${FUZZCFG:-c3d1}   # FUZZCFG=d1g などで設定を選ぶ (cfg.py の CFGS)
L=analysis/smallfix_20261113/fuzz_logs; [ $CFG != c3d1 ] && L=${L}_$CFG; mkdir -p $L
ENVS=$(python3 -c "import sys; sys.path.insert(0, 'analysis/smallfix_20261113'); from cfg import CFGS; import os; print(' '.join(f'{k}={v}' for k, v in CFGS[os.environ.get('FUZZCFG', 'c3d1')].items()))")
STRESS="ENOMOTO_MIP_CLQFB_CAP=it ENOMOTO_T_MIP_CLQFB_CAP_MINIT=1 ENOMOTO_T_MIP_CLQFB_CAP_MULT=0 ENOMOTO_T_MIP_CLQFB_CAP_MIN=0 ENOMOTO_T_MIP_TRNOFIX_FRAC=1"
run() { name=$1; shift; echo "== $name: $*"; t0=$(date +%s); env $ENVS "$@" > $L/$name.log 2>&1; echo "rc=$? $(( $(date +%s) - t0 ))s"; tail -5 $L/$name.log; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run stress_tiny env $STRESS ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run stress_highs env $STRESS ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
CT="cargo test --release --lib"
export CARGO_TARGET_DIR=target/test-py RUSTFLAGS="-L /usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib
echo "== cargo test (既定)"; env $CT > $L/cargo_def.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_def.log
echo "== cargo test ($CFG)"; env $ENVS $CT > $L/cargo_$CFG.log 2>&1; echo "rc=$?"; grep "test result" $L/cargo_$CFG.log
