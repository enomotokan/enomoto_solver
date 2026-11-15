#!/bin/bash
cd "$(dirname "$0")"
while ! grep -q CHAIN2_DONE logs/chain2.out; do sleep 30; done
python bench.py r600.jsonl 600 neos-1456979 def,h 1,2
echo CHAIN3_DONE
