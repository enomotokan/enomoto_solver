#!/bin/bash
# run.sh <name> <problem> <T> <outlog>   (4 スレッド。追加の環境変数はそのまま渡る)
name=$1; p=$2; T=$3; out=$4
export PYTHONPATH=/home/user/wt/b/$name/.pylib ENOMOTO_MIP_THREADS=4
cd /tmp && python /home/user/wt/scripts/run1.py /home/user/enomoto_solver/.miplib_cache/$p.mps $T 4 > $out 2>&1
