#!/bin/bash
# 計測の待ち行列 (2026-11-07、HNFCUT): rococo の 600 秒 → 58 問の 60 秒 (種 1・2)。止まったら同じ命令で続きから回る
cd "$(dirname "$0")"
python run.py long600.jsonl 600 def,hnf,hcut,hcut_b,hcut_ac 1,2 rococoB10-011000 rococoC10-001000 > long600.log 2>&1
python run.py p58.jsonl 60 def,hcut 1,2 @p58 > p58.log 2>&1
echo done > chain_done
