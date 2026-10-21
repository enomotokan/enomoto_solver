#!/bin/bash
# run.sh <name> <tag> <T> [env...] : ENOMOTO を走らせてログを logs/<name>_<tag>.log に
d=$(cd $(dirname $0) && pwd); n=$1; tag=$2; T=$3; shift 3
cd /tmp && env ENOMOTO_MIP_LOG=1 "$@" python $d/run_eno.py /home/user/enomoto_solver/.miplib_cache/$n.mps.gz $T > $d/logs/${SUB:-}${n}_${tag}.log 2>&1
grep RESULT $d/logs/${SUB:-}${n}_${tag}.log | cut -c1-300
