#!/bin/bash
# 58 問で差の出た問題の切り分け (60 秒、試作を 1 つずつ)
cd "$(dirname "$0")"
r() { python3 run1.py ours "$@"; }
for s in 1 2; do
  r misc07 a_def $s 60
  r misc07 a_rw $s 60 ENOMOTO_MIP_S1_RINSWIDE=2
  r misc07 a_ipf $s 60 ENOMOTO_MIP_S1_IPMFIND=1
  r misc07 a_fbr $s 60 ENOMOTO_T_MIP_CLIQUE_FB_RATIO=0.1
  r timtab1 a_def $s 60
  r timtab1 a_rw $s 60 ENOMOTO_MIP_S1_RINSWIDE=2
  r timtab1 a_fbr $s 60 ENOMOTO_T_MIP_CLIQUE_FB_RATIO=0.1
done
echo ALL_DONE
