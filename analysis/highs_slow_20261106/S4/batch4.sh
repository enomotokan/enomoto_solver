#!/bin/bash
# 組み直した切り分け (2026-11-06): neos-957323 の対称性 (G6 の試作)・HiGHS の対称性なし、cod105 の根の RINS、mzzv42z の前処理と再スタート後の根
cd "$(dirname "$0")"
SYM="ENOMOTO_MIP_HO6_SYMWORK=1 ENOMOTO_MIP_HO6_SYMREFINE=1 ENOMOTO_T_MIP_PRESOLVE_WORK_SYM=5e8 ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG=1e9 ENOMOTO_T_MIP_SYM_TIME=1"
python3 run1.py sym_s0 neos-957323 ours 0 $SYM
python3 run1.py highs_nosym neos-957323 highs 0 ENOMOTO_BENCH_HIGHS_OPT=mip_detect_symmetry=false
python3 run1.py sym_s1 neos-957323 ours 1 $SYM
python3 run1.py sym_s2 neos-957323 ours 2 $SYM
for s in 0 1 2; do python3 run1.py norins_s$s cod105 ours $s ENOMOTO_MIP_NO_ROOT_RINS=1; done
python3 run1.py nosol50_s0 cod105 ours 0 ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP=50
python3 run1.py nolpp_s0 cod105 ours 0 ENOMOTO_MIP_NO_LP_PROBE=1
for s in 0 1 2; do python3 run1.py rfeqff_s$s mzzv42z ours $s ENOMOTO_MIP_CLIQUE_EXTEND_ROWS_FIRST=1 ENOMOTO_MIP_PROBE_EQUIV_FIX=1 ENOMOTO_MIP_GR_RESTART=ff; done
for s in 0 1 2; do python3 run1.py allh_s$s mzzv42z ours $s ENOMOTO_MIP_RESTART_ALL_HEURS=1; done
