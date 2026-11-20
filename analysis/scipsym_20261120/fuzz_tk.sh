#!/bin/bash
# 試作 ENOMOTO_MIP_LPPROBE_TICKS=1 の誤答の確かめ。リポジトリの根から: bash analysis/scipsym_20261120/fuzz_tk.sh → fuzz/tk_*.txt
# .cov は LP による固定で試作があきらめた求解の数 (ログの「LPPROBE_TICKS gave up」)。
set -u
cd "$(dirname "$0")/../.."
F=analysis/scipsym_20261120/fuzz; mkdir -p $F
run() { name=tk_$1; shift; env ENOMOTO_MIP_LPPROBE_TICKS=1 ENOMOTO_MIP_LOG=1 "$@" 2> $F/$name.err | grep -E "^(seed|bad)" | tail -20 > $F/$name.txt; echo "$(grep -c 'LPPROBE_TICKS gave up' $F/$name.err) gave up / $(grep -c 'LP probing: root LP' $F/$name.err) LP probings" > $F/$name.cov; rm -f $F/$name.err; echo "$name: $(tail -1 $F/$name.txt) / $(cat $F/$name.cov)"; }
run vs_highs env ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300 2000
run vs_highs_t1 env ENOMOTO_MIP_THREADS=1 ENOMOTO_MIP_REL_GAP=1e-9 python3 scripts/mip_fuzz_vs_highs.py 300 3000
run fixed_charge python3 scripts/mip_fuzz_fixed_charge.py 300
run clique python3 scripts/mip_fuzz_clique.py 300
run sym1 python3 scripts/mip_fuzz_symmetric.py 300 900 1
run pplex python3 analysis/scipsym_20261120/fuzz_pplex.py 300 9000
