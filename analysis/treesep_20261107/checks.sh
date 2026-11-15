#!/bin/bash
# 誤答の検査 (試作の経路を強制的に通す設定) と cargo test
cd "$(dirname "$0")/../.."
export ENOMOTO_MIP_TREESEP2=abcd ENOMOTO_T_MIP_TREESEP2_GAP=1 ENOMOTO_T_MIP_TREESEP2_NEAR=1 ENOMOTO_T_MIP_TREESEP2_AGE=2 ENOMOTO_T_MIP_TREESEP2_ROWS=0.01
D=analysis/treesep_20261107
python $D/fuzz_tree.py 300 0 2>/dev/null | grep -E "MISM|done" > $D/check_fuzz_tree.txt
python benchmarks/equiv2_bug_20261022/fuzz_tiny.py 400 0 2>/dev/null | tail -3 > $D/check_fuzz_tiny.txt
python scripts/mip_fuzz_vs_highs.py 300 2>/dev/null | tail -3 > $D/check_fuzz_highs.txt
export ENOMOTO_MIP_THREADS=4
python $D/fuzz_tree.py 100 1000 2>/dev/null | grep -E "MISM|done" > $D/check_fuzz_tree_t4.txt
unset ENOMOTO_MIP_TREESEP2 ENOMOTO_MIP_THREADS
RUSTFLAGS="-C link-arg=-L/usr/local/lib -C link-arg=-lpython3.13" LD_LIBRARY_PATH=/usr/local/lib cargo test --release --lib 2>&1 | grep -E "^test result|FAILED|panicked" > $D/check_cargo_test.txt
echo finished >> $D/check_cargo_test.txt
