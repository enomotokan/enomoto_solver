#!/bin/bash
# step2b の 600 秒の種 2 の続き (コンテナの入れ替わりで止まった分。res.jsonl にある回は飛ばす)
cd "$(dirname "$0")/../.."
R=analysis/tree_restart_guard_20261106/run1.py
J=analysis/tree_restart_guard_20261106/res.jsonl
export RUN_TL=600
for p in pg5_34 swath3 neos-3627168-kasai csched008 sct2 uccase12 drayage-25-23 mc11 neos-860300 eil33-2; do
  grep -q "\"problem\": \"$p\", \"seed\": \"2\", \"tag\": \"x_def\"" $J || python $R $p 2 x_def
  grep -q "\"problem\": \"$p\", \"seed\": \"2\", \"tag\": \"x_forcekC\"" $J || python $R $p 2 x_forcekC ENOMOTO_MIP_TREE_RESTART=force ENOMOTO_MIP_TREE_RESTART_GUARD=kC
done
