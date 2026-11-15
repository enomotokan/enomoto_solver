#!/bin/bash
# 誤答の検査 (analysis/modkvb_20261107.md): 既存の 3 つと fuzz_vb.py を mk (ENOMOTO_MIP_MODKVB=1) で (fuzz_vb.py は既定も)
cd "$(dirname "$0")"; WT=$(cd ../../..; pwd); F=analysis/modkvb_20261107/fuzz
E="ENOMOTO_MIP_MODKVB=1 ENOMOTO_MIP_MODKVB_LOG=1"
[ -s tiny_mk.txt ] || (cd $WT && env $E nice python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > $F/tiny_mk.txt 2> $F/tiny_mk.err)
[ -s vs_highs_mk.txt ] || (cd $WT && env $E ENOMOTO_MIP_REL_GAP=1e-9 nice python scripts/mip_fuzz_vs_highs.py 300 > $F/vs_highs_mk.txt 2> $F/vs_highs_mk.err)
[ -s fuzz_mush_mk.txt ] || env $E nice python $WT/analysis/mushroom_cuts_20261106/fuzz_mush.py 3000 1000 > fuzz_mush_mk.txt 2> fuzz_mush_mk.err
[ -s fuzz_vb_mk.txt ] || env $E nice python ../fuzz_vb.py 3000 0 > fuzz_vb_mk.txt 2> fuzz_vb_mk.err
[ -s fuzz_vb_def.txt ] || nice python ../fuzz_vb.py 3000 0 > fuzz_vb_def.txt 2> fuzz_vb_def.err
for f in *.err; do echo "$f modkvb lines $(grep -c 'SEP modkvb' $f) with cands $(grep 'SEP modkvb' $f | grep -vc 'cands 0$')"; done > counts.txt
for f in *.txt; do [ "$f" = counts.txt ] || { grep -E "^seed|^bad|bad " $f > ${f%.txt}.summ; }; done
gzip -f *.err
