#!/bin/bash
# 試作 ENOMOTO_MIP_PRBWORK の誤答の確かめ (release019_20261117/fuzz.sh の写し)。小さい問題でも持ち分が尽きるよう、
# 持ち分 (= 何も見つからない間の上限 ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0) を 1 と 1e5 に下げて回す
# (1 なら最初のプロービングで尽き、再スタートの後は必ずプロービングを省いて含意を引き継ぐ)。
# リポジトリの根から: bash analysis/probework_20261120/fuzz.sh   ログは analysis/probework_20261120/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/probework_20261120/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name"; "$@" > $L/$name.log 2>&1; echo "rc=$?"; grep -E "bad|mismatch|wrong" $L/$name.log | tail -1; tail -1 $L/$name.log; }
export ENOMOTO_MIP_PRBWORK=1
export ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0=1
run tiny_w1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run highs_w1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run objint_w1 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run slack_w1 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
export ENOMOTO_T_MIP_PRESOLVE_WORK_PROBE0=1e5
run tiny_w5 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run highs_w5 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 150
run objint_w5 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
echo FUZZ_DONE
