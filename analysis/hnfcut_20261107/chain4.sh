#!/bin/bash
# 待ち行列 (4 回目、2026-11-07): chain3 の終わりを待つ → avoca・piperout-d20/d27 の 300 秒 (種 1・2)
cd "$(dirname "$0")"
while [ ! -f chain3_done ]; do sleep 30; done
python run.py long300.jsonl 300 def,hcut_b,hcut_c,hcut 1,2 neos-3373491-avoca piperout-d20 piperout-d27 > long300.log 2>&1
echo done > chain4_done
