#!/bin/bash
# 試作 POLISH_STRICT つきの combo_t1・def_t1 と、HiGHS・SCIP を 600 秒ずつ順に解き、返す値を check.py・fixlp.py で確かめる
cd "$(dirname "$0")"
python3 solve.py combo_t1 600 ENOMOTO_MIP_POLISH_STRICT=1 > log_combo_t1_strict.txt 2>&1
python3 solve.py def_t1 600 ENOMOTO_MIP_POLISH_STRICT=1 > log_def_t1_strict.txt 2>&1
python3 others.py highs 600 > log_highs.txt 2>&1
python3 others.py scip 600 > log_scip.txt 2>&1
for f in x_combo_t1_POLISH_STRICT1.json x_def_t1_POLISH_STRICT1.json x_highs.json x_scip.json; do python3 check.py $f; python3 fixlp.py $f; done
