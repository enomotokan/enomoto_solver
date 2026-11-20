#!/bin/bash
# 試作 ENOMOTO_MIP_PROBE_USELESS の誤答の確かめ (release019_20261117/fuzz.sh の写し)。小さい問題でも打ち切りが起きるよう K = 1 と K = 50 で回す。
# リポジトリの根から: bash analysis/scipslow_20261119/fuzz.sh   ログは analysis/scipslow_20261119/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/scipslow_20261119/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatches" $L/$name.log | tail -1; }
export ENOMOTO_MIP_PROBE_USELESS=1
run tiny_k1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run highs_k1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run objint_k1 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run slack_k1 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
export ENOMOTO_MIP_PROBE_USELESS=50
run tiny_k50 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run highs_k50 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 150
echo FUZZ_DONE
