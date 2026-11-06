#!/bin/bash
# 本計測: 問題ごとに 既定 (def)・その問題の試作・3 つ全部 (s1all) を種 1〜3 で交互に (上限 300 秒)
cd "$(dirname "$0")"
r() { python3 run1.py ours "$@"; }
ALL="ENOMOTO_MIP_S1_RINSWIDE=2 ENOMOTO_MIP_S1_IPMFIND=1 ENOMOTO_T_MIP_CLIQUE_FB_RATIO=0.1"
for s in 1 2 3; do
  r neos-860300 m_def $s 300
  r neos-860300 m_fbr $s 300 ENOMOTO_T_MIP_CLIQUE_FB_RATIO=0.1
  r neos-860300 m_all $s 300 $ALL
  r hypothyroid-k1 m_def $s 300
  r hypothyroid-k1 m_ipf $s 300 ENOMOTO_MIP_S1_IPMFIND=1
  r hypothyroid-k1 m_all $s 300 $ALL
  r physiciansched6-2 m_def $s 300
  r physiciansched6-2 m_rw $s 300 ENOMOTO_MIP_S1_RINSWIDE=2
  r physiciansched6-2 m_all $s 300 $ALL
done
echo ALL_DONE
