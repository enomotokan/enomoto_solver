#!/bin/bash
# P2-INF: cfx2 (LIT_EXPLAIN・HIGHS_CONFLICT_SCORE・擬似費用に手がかりがないときだけ衝突の重み 0.1) の誤答の確かめ。リポジトリの根から: PYTHONPATH=<既定化した版> bash analysis/p2inf_20261119/verify_cfx2.sh
# widden (最適 25009.662227) を 1・4 スレッド × 種 1 で 1 回ずつ (デバッグ解なし)、fuzz 8 本。ログは fuzz_logs/cfx2_*.txt。
cd "$(dirname "$0")/../.."
export ENOMOTO_MIP_LIT_EXPLAIN=1 ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1 ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT=0.1
D=analysis/p2inf_20261119; L=$D/fuzz_logs; mkdir -p $L
for th in 4 1; do TL=900 python3 $D/wdn.py neos-4722843-widden cfx2_t$th 1-1 25009.662227 $th ENOMOTO_MIP_LIT_EXPLAIN=1 ENOMOTO_MIP_HIGHS_CONFLICT_SCORE=1 ENOMOTO_T_MIP_CONFLICT_WEIGHT_FLAT=0.1 > $L/cfx2_widden_t$th.txt 2>&1; done
run() { name=$1; shift; "$@" > $L/cfx2_$name.txt 2>&1; echo "== $name rc=$?"; grep -E "^bad|mismatches" $L/cfx2_$name.txt | tail -1; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_highs_t1 env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run fuzz_cutconf python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 0
run fuzz_symmetric python3 scripts/mip_fuzz_symmetric.py 300
run fuzz_cutconf2 python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 1000
echo done > $L/cfx2_done.txt
