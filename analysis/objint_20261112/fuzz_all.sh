#!/bin/sh
# OBJINT の誤答の検査をまとめて回す (結果は fuzz/*.txt)
cd "$(dirname "$0")"; mkdir -p fuzz
python fuzz_objint.py 1000 0 > fuzz/objint_def.txt 2>&1
ENOMOTO_MIP_OBJINT=1 python fuzz_objint.py 1000 0 > fuzz/objint_on.txt 2>&1
ENOMOTO_MIP_OBJINT=1 python ../../benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > fuzz/tiny_on.txt 2>&1
ENOMOTO_MIP_OBJINT=1 ENOMOTO_MIP_REL_GAP=1e-9 python ../../scripts/mip_fuzz_vs_highs.py 300 > fuzz/vs_highs_on.txt 2>&1
ENOMOTO_MIP_OBJINT=1 python ../implint_tu_20261110/fuzz_tu.py 1000 0 > fuzz/tu_on.txt 2>&1
