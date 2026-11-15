#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
while pgrep -f q1.sh >/dev/null; do sleep 20; done
./run1.sh highs fastxgemm-n2r6s0t2 hinosym ENOMOTO_BENCH_HIGHS_OPT=mip_detect_symmetry=false
