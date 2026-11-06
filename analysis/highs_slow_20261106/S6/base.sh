#!/bin/bash
# 既定 2 回 (種 1・2) と HiGHS 1 回 (ログ付き) を 600 秒で。前の 600 秒の比べの 1 回と合わせて 3 回・2 回にする
cd "$(dirname "$0")"
for p in nexp-150-20-8-5 neos-662469 trento1 thor50dday; do
  python3 run1.py def $p 600 1
  python3 run1.py highs $p 600 0
  python3 run1.py def $p 600 2
done
