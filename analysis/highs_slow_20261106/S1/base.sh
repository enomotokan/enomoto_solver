#!/bin/bash
# 基準の計測: 各問題 ENOMOTO 既定 3 回 (種 1-3)・HiGHS 2 回、上限 300 秒
cd "$(dirname "$0")"
for p in neos-860300 hypothyroid-k1 physiciansched6-2; do
  python3 run1.py highs $p highs 1 300
  for s in 1 2 3; do python3 run1.py ours $p def $s 300; done
  python3 run1.py highs $p highs 2 300
done
echo ALL_DONE
