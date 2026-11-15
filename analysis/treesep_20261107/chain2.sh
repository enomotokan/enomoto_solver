#!/bin/bash
cd "$(dirname "$0")"
while pgrep -f chain1.sh > /dev/null; do sleep 30; done
export TSB_DIAG=5
python bench.py mech2.jsonl 120 neos5,misc07,mik-250-20-75-4 a,ab,abd 1,2 >> mech2.out 2>&1
