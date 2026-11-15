#!/bin/bash
# 待ち行列 (3 回目、2026-11-07): chain2 の終わりを待つ → 探索の当たりの根の比べ → 根の LP に届かなかった問題の診断 (120 秒)
cd "$(dirname "$0")"
while [ ! -f chain2_done ]; do sleep 30; done
python run.py root60c.jsonl 60 def,hcut_b,hcut_c,hcut 1 @hits3 > root60c.log 2>&1
python run.py diag120.jsonl 120 hcut_b 1 @unreached > diag120.log 2>&1
echo done > chain3_done
