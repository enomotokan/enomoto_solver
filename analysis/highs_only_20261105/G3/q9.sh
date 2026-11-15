#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
until [ -e q8.done ]; do sleep 20; done
./run1.sh ours fastxgemm-n2r6s0t2 symnorm2 ENOMOTO_MIP_HO3_SYMNORM=1
./run1.sh ours fastxgemm-n2r6s0t2 def2
touch q9.done
