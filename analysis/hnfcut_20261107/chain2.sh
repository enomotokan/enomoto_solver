#!/bin/bash
# 待ち行列 (2 回目、2026-11-07): collection の探索の続き → ns1631475 の 300 秒
cd "$(dirname "$0")"
(cd scan && python ../scan.py ../scan.jsonl 20 40 >> ../scan.log 2>&1)
python run.py ns300.jsonl 300 def,hcut_b,hcut_c 1 ns1631475 > ns300.log 2>&1
echo done > chain2_done
