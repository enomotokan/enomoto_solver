#!/bin/bash
# 改良 (11-08、whl6 = 11-07 の最新 + R・K、forceKR2 は whl7 (R の止まりの判定を Δq < 1e-3 に)): 短い 6 問を 60 秒・種 1〜3 で def・forcekC・forceKR (res.jsonl にある回は飛ばす)
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
J=analysis/tree_restart_guard_20261106/res.jsonl
run() { grep -q "\"problem\": \"$1\", \"seed\": \"$2\", \"tag\": \"$3\"" $J || python $R "$@"; }
for s in 1 2 3; do
for p in eil33-2 neos-860300 mc11 drayage-25-23 neos5 pk1; do
  run $p $s y_def
  run $p $s y_forcekC ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kC
  run $p $s y_forceKR2 ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=KR
done; done
