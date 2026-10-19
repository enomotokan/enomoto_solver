#!/bin/bash
# 4 スレッドの計測をまとめて 1 つずつ実行 (各ソルバーごとに with_slot)
cd /home/user/wt/mzzv42z_neos960392/analysis/mzzv
export ENOMOTO_MIP_THREADS=4 PYTHONPATH=/home/user/wt/mzzv42z_neos960392/.pylib
SOLOUT=sol_neos-960392.txt /tmp/with_slot.sh python run_highs.py neos-960392 60 > highs4_neos-960392.log 2>&1
ENOMOTO_MIP_LOG=1 /tmp/with_slot.sh python run_eno.py neos-960392 60 > eno4_neos-960392_a.log 2>&1
SOLOUT=sol_mzzv42z.txt /tmp/with_slot.sh python run_highs.py mzzv42z 120 > highs4_mzzv42z.log 2>&1
ENOMOTO_MIP_LOG=1 /tmp/with_slot.sh python run_eno.py mzzv42z 60 > eno4_mzzv42z_a.log 2>&1
