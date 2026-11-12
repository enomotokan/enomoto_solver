#!/bin/sh
# OBJINT の `c` の誤答の検査 (結果は fuzz/*_c.txt)
cd "$(dirname "$0")"; mkdir -p fuzz
ENOMOTO_MIP_OBJINT=hfc python fuzz_objint.py 3000 0 > fuzz/objint_hfc.txt 2>&1
python fuzz_objint.py 3000 0 > fuzz/objint_def_c.txt 2>&1
ENOMOTO_MIP_OBJINT=hfc python ../../benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > fuzz/tiny_hfc.txt 2>&1
ENOMOTO_MIP_OBJINT=hfc ENOMOTO_MIP_REL_GAP=1e-9 python ../../scripts/mip_fuzz_vs_highs.py 300 > fuzz/vs_highs_hfc.txt 2>&1
ENOMOTO_MIP_OBJINT=hfc python ../implint_tu_20261110/fuzz_tu.py 1000 0 > fuzz/tu_hfc.txt 2>&1
