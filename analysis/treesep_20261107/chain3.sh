#!/bin/bash
cd "$(dirname "$0")"
TSB_DIAG=30 python bench.py tgt600.jsonl 600 tgt b 1 >> chain3.out 2>&1
python bench.py b58.jsonl 60 p58 def,b 1,2 >> chain3.out 2>&1
TSB_DIAG=30 python bench.py tgt600.jsonl 600 tgt def,b 2 >> chain3.out 2>&1
