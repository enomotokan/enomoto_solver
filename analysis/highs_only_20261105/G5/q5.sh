#!/bin/bash
cd $(dirname $0)
until grep -q Q4DONE q.log; do sleep 20; done
while [ -f /tmp/G5_build.lock ]; do sleep 5; done
python3 hsol.py /home/user/enomoto_solver/.miplib_cache/traininstance6.mps sol_traininstance6.txt 600 > logs/hsol_traininstance6.log 2>&1
echo Q5DONE >> q.log
