#!/bin/bash
# run.sh <pylib or "sys"> <problem> <T> <threads> <outlog>
lib=$1; p=$2; T=$3; th=$4; out=$5
envs=""
[ "$lib" != "sys" ] && export PYTHONPATH=$lib
if [ "$th" -gt 1 ]; then export ENOMOTO_MIP_THREADS=$th; fi
cd /tmp && python /home/user/wt/regress1-scripts/run1.py /home/user/enomoto_solver/.miplib_cache/$p.mps $T $th > $out 2>&1
