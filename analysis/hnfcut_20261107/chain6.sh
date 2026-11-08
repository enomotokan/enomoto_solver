#!/bin/bash
# 待ち行列 (改良 11-08): chain5 の後に 58 問 (60 秒・種 1・2) で既定と HNFCUT=bc
cd "$(dirname "$0")"
while [ ! -f chain5_done ]; do sleep 30; done
python run.py v2_p58.jsonl 60 def,hcut_bc 1,2 @p58 > v2_p58.log 2>&1
echo done > chain6_done
