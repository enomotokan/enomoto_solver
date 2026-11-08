#!/bin/bash
# 計測の連鎖 (ソルバーは同時に 1 つ)。途中で止まっても同じ順で回し直せば続きから。
cd "$(dirname "$0")"
while pgrep -f "ablq2.sh" >/dev/null; do sleep 10; done
P600=neos-3627168-kasai,csched008,unitcal_7,tr12-30,sct2,neos-1456979,rococoC10-001000
python bench.py e600.jsonl 600 $P600 def,h 1
export ENOMOTO_MIP_REL_GAP=1e-9
for cfg in "" "dh"; do
  if [ -n "$cfg" ]; then export ENOMOTO_MIP_NODECNT=$cfg; else unset ENOMOTO_MIP_NODECNT; fi
  tag=${cfg:-def}
  [ -s logs/fuzz_tiny_$tag.out ] || python ../../benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > logs/fuzz_tiny_$tag.out 2>/dev/null
  [ -s logs/fuzz_vs_$tag.out ] || python ../../scripts/mip_fuzz_vs_highs.py 300 > logs/fuzz_vs_$tag.out 2>/dev/null
  [ -s logs/fuzz_fc_$tag.out ] || python fuzz_fc.py 300 1000 > logs/fuzz_fc_$tag.out 2>/dev/null
done
unset ENOMOTO_MIP_NODECNT ENOMOTO_MIP_REL_GAP
python bench.py p58.jsonl 60 p58 def,h 1,2
python bench.py e600.jsonl 600 $P600 def,h 2
echo CHAIN_DONE
