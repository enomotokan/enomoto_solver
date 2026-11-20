#!/bin/bash
# P2-INF: ROWTINY の既定化の確かめ (親の依頼)。リポジトリの根から: PYTHONPATH=<既定化した版> bash analysis/p2inf_20261119/verify_adopt.sh
# widden (最適 25009.662227) を 1・4 スレッド × 種 1 で 1 回ずつ (デバッグ解なし)、fuzz 8 本。ログは fuzz_logs/adopt_rt_*.txt。
cd "$(dirname "$0")/../.."
D=analysis/p2inf_20261119; L=$D/fuzz_logs; mkdir -p $L
for th in 4 1; do TL=900 python3 $D/wdn.py neos-4722843-widden adopt_rt_t$th 1-1 25009.662227 $th > $L/adopt_rt_widden_t$th.txt 2>&1; done
run() { name=$1; shift; "$@" > $L/adopt_rt_$name.txt 2>&1; echo "== $name rc=$?"; grep -E "^bad|mismatches" $L/adopt_rt_$name.txt | tail -1; }
run fuzz_tiny env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0
run fuzz_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_highs_t1 env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300
run fuzz_objint python3 analysis/objint_20261112/fuzz_objint.py 1000 0
run fuzz_slack python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 0
run fuzz_tiny2 env ENOMOTO_MIP_REL_GAP=1e-9 python3 benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 1000
run fuzz_objint2 python3 analysis/objint_20261112/fuzz_objint.py 1000 5000
run fuzz_slack2 python3 analysis/wrongans_20261112/seed14/fuzz_slack.py 1000 5000
run fuzz_cutconf python3 analysis/cutconf_20261110/fuzz_cutconf.py 300 0
echo done > $L/adopt_rt_done.txt
