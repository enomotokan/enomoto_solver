#!/bin/bash
cd "$(dirname "$0")"
export TSB_DIAG=5
python bench.py mech.jsonl 120 neos5,misc07,mik-250-20-75-4 def,nots,every,a 1,2 >> mech.out 2>&1
export TSB_DIAG=30
python bench.py tgt600.jsonl 600 tgt def 1 >> tgt600.out 2>&1
