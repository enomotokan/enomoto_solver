#!/bin/bash
# go.sh <tag> <T> [env...] : ENOMOTO を 4 スレッドで
tag=$1; T=$2; shift 2
cd /tmp && env ENOMOTO_MIP_THREADS=${THR:-4} THR=${THR:-4} "$@" python /home/user/enomoto_solver/runs_mc11/en.py /home/user/enomoto_solver/.miplib_cache/mc11.mps.gz $T > /home/user/enomoto_solver/runs_mc11/logs/$tag.log 2>&1
