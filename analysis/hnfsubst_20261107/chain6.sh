#!/bin/bash
# 改良 (11-08) の計測の順 その 5: (chain5 の後) 今の wheel の既定の 600 秒 (HNFBR の比べの相手。long600 の def は前の wheel)
cd "$(dirname "$0")"
while pgrep -f "chain[45].sh" >/dev/null; do sleep 20; done
python run.py long600b.jsonl 600 def 1,2 mzzv11 rococoB10-011000 rococoC10-001000 traininstance2
