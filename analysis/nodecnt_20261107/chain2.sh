#!/bin/bash
cd "$(dirname "$0")"
python bench.py r600.jsonl 600 neos-3627168-kasai,neos-1456979,csched008 hr 1,2
python bench.py r58.jsonl 60 p58 def,hr 1,2
export ENOMOTO_MIP_REL_GAP=1e-9 ENOMOTO_MIP_NODECNT=dhr
python fuzz_fc.py 300 1000 2>/dev/null | grep -E '^(done|MISMATCH)' > logs/fuzz_fc_dhr.txt
python ../../scripts/mip_fuzz_vs_highs.py 300 2>/dev/null | grep -E '^(bad|seed)' > logs/fuzz_vs_dhr.txt
python ../../benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 2>/dev/null | grep -E '^(bad|seed)' > logs/fuzz_tiny_dhr.txt
echo CHAIN2_DONE
