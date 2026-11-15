#!/bin/bash
# 改良 (11-08、whl7): 600 秒の 5 問 (あと一歩の問題) を def・forceKR で種 1・2 → 58 問 (60 秒、種 1・2) → 誤答の検査 → 600 秒の種 3・4。
# res.jsonl・bench58_w7.jsonl・fuzz/ にあるものは飛ばす (コンテナの入れ替わりで止まったら、もう一度この台本を起こせば続きから)
cd "$(dirname "$0")/../.."
D=analysis/tree_restart_guard_20261106
R=$D/run1.py
J=$D/res.jsonl
run() { grep -q "\"problem\": \"$1\", \"seed\": \"$2\", \"tag\": \"$3\"" $J || python $R "$@"; }
long() {
  export RUN_TL=600
  for p in pg5_34 swath3 neos-3627168-kasai sct2 uccase12; do
    run $p $1 z_def
    run $p $1 z_forceKR ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=KR
  done
  unset RUN_TL
}
long 1; long 2
export BENCH58_OUT=bench58_w7.jsonl
python $D/bench58.py def,forceKR 1
python $D/bench58.py def,forceKR 2
unset BENCH58_OUT
E="ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=KR"
[ -s $D/fuzz/tiny_def.txt ] || python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > $D/fuzz/tiny_def.txt 2> /dev/null
[ -s $D/fuzz/tiny_KR.txt ] || env $E python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > $D/fuzz/tiny_KR.txt 2> /dev/null
[ -s $D/fuzz/vs_highs_KR.txt ] || env $E ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300 > $D/fuzz/vs_highs_KR.txt 2> /dev/null
long 3; long 4
