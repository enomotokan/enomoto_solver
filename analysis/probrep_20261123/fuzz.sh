#!/bin/bash
# 試作 ENOMOTO_MIP_PROBREP の誤答の確かめ (probework_20261120/fuzz.sh と同じ 7 本)。小さい問題では成果 0 の回の後に列の変わらない回が少ないので、
# 確かめ用の値 `f` (成果があっても回の写しを控える) で、前の回の含意を使い回して省く道を通す。
# ENOMOTO_MIP_LOG=1 で、省いた回 (skipped (PROBREP) をログで数える。
# リポジトリの根から: bash analysis/probrep_20261123/fuzz.sh [値]   (既定の値 f)。ログは analysis/probrep_20261123/fuzz_logs/ (コミットしない)
set -u
cd "$(dirname "$0")/../.."
V=${1:-f}
L=analysis/probrep_20261123/fuzz_logs; mkdir -p $L
run() { name=$1_$V; shift; echo "== $name"; "$@" > $L/$name.log 2>&1; echo "rc=$? skipped=$(grep -c 'skipped (PROBREP' $L/$name.log)"; grep -E "^bad|mismatches" $L/$name.log | tail -1; }
export ENOMOTO_MIP_PROBREP=$V ENOMOTO_MIP_LOG=1
run tiny0 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run tiny1 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 600
run objint0 python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run objint1 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run highs2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 1500 10000
run slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
echo FUZZ_DONE
