#!/bin/bash
# HNFCUT の誤答の検査 (2026-11-07)。ENOMOTO_MIP_HNFCUT=1 で 3 種の乱数の問題を HiGHS と突き合わせる
cd "$(dirname "$0")/../.."
export ENOMOTO_MIP_HNFCUT=1 ENOMOTO_MIP_HNFPRE_QUIET=1
python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 > analysis/hnfcut_20261107/fuzz_tiny_hcut.txt 2>&1
python scripts/mip_fuzz_vs_highs.py 300 > analysis/hnfcut_20261107/fuzz_highs_hcut.txt 2>&1
python analysis/hnfpre_20261107/fuzz_hnf.py 1000 0 > analysis/hnfcut_20261107/fuzz_hnf_hcut.txt 2>&1
echo done > analysis/hnfcut_20261107/fuzz_done
