#!/bin/bash
# 誤答の検査 (analysis/mushroom_cuts_20261106.md): 既存の 2 つと fuzz_mush.py を mc (ENOMOTO_MIP_MUSHAGG=cover) と m1 (=1) で
cd "$(dirname "$0")"; WT=$(cd ../../..; pwd)
for c in mc:cover m1:1; do
  n=${c%%:*}; v=${c#*:}
  (cd $WT && env ENOMOTO_MIP_MUSHAGG=$v ENOMOTO_MIP_MUSHAGG_LOG=1 python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > analysis/mushroom_cuts_20261106/fuzz/tiny_$n.txt 2> analysis/mushroom_cuts_20261106/fuzz/tiny_$n.err)
  (cd $WT && env ENOMOTO_MIP_MUSHAGG=$v ENOMOTO_MIP_MUSHAGG_LOG=1 ENOMOTO_MIP_REL_GAP=1e-9 python scripts/mip_fuzz_vs_highs.py 300 > analysis/mushroom_cuts_20261106/fuzz/vs_highs_$n.txt 2> analysis/mushroom_cuts_20261106/fuzz/vs_highs_$n.err)
  env ENOMOTO_MIP_MUSHAGG=$v ENOMOTO_MIP_MUSHAGG_LOG=1 python ../fuzz_mush.py 3000 1000 > fuzz_mush_$n.txt 2> fuzz_mush_$n.err
done
for f in *.err; do grep -c "SEP mushagg" $f | sed "s|^|$f mushagg lines |"; grep "SEP mushagg" $f | grep -vc "cands 0$" | sed "s|^|$f with cands |"; gzip -f $f; done > counts.txt
