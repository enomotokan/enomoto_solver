#!/bin/bash
# 既定と HiGHS の比べ (各問題 HiGHS 2 回・ENOMOTO 種 0〜2)
cd "$(dirname "$0")"
for p in mzzv42z neos-957323 cod105; do
  python3 run1.py highs1 $p highs 0
  python3 run1.py def_s0 $p ours 0
  python3 run1.py highs2 $p highs 0
  python3 run1.py def_s1 $p ours 1
  python3 run1.py def_s2 $p ours 2
done
