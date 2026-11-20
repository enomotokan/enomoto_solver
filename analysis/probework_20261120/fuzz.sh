#!/bin/bash
# 試作 ENOMOTO_MIP_PRBWORK の誤答の確かめ (release019_20261117/fuzz.sh の写し)。小さい問題は候補が 16 未満で打ち切りの判定に届かないので、
# 確かめ用の値 `f` (成果 0 なら打ち切りによらず印を付ける) で、再スタートの後のプロービングを省いて含意を引き継ぐ道を通す。
# ENOMOTO_MIP_LOG=1 で、省いた回 (skipped after the restart) をログで数える。
# リポジトリの根から: bash analysis/probework_20261120/fuzz.sh   ログは analysis/probework_20261120/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
L=analysis/probework_20261120/fuzz_logs; mkdir -p $L
run() { name=$1; shift; echo "== $name"; "$@" > $L/$name.log 2>&1; echo "rc=$? skipped_after_restart=$(grep -c 'skipped after the restart' $L/$name.log)"; grep -E "^bad|mismatches" $L/$name.log | tail -1; }
export ENOMOTO_MIP_PRBWORK=f ENOMOTO_MIP_LOG=1
run tiny_f0 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run tiny_f1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run highs_f env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 600
run objint_f0 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run objint_f1 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run highs_f2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 1500 10000
run slack_f python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
echo FUZZ_DONE
