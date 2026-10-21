#!/bin/bash
# gp.sh <problem> <tag> <T> [env...] : 任意の問題を ENOMOTO 4 スレッドで (ログは logs/side/<problem>_<tag>.log)
prob=$1; tag=$2; T=$3; shift 3
mkdir -p /home/user/enomoto_solver/runs_mc11/logs/side
cd /tmp && env ENOMOTO_MIP_THREADS=4 THR=4 "$@" python /home/user/enomoto_solver/runs_mc11/en.py /home/user/enomoto_solver/.miplib_cache/$prob.mps.gz $T > /home/user/enomoto_solver/runs_mc11/logs/side/${prob}_$tag.log 2>&1
