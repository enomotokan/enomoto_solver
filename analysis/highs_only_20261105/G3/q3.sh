#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
while pgrep -f "q[12].sh" >/dev/null; do sleep 20; done
./run1.sh ours fastxgemm-n2r6s0t2 symnorm ENOMOTO_MIP_HO3_SYMNORM=1
