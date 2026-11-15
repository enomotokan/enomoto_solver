#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
while pgrep -f "q[12].sh" >/dev/null; do sleep 20; done
./run1.sh highs rococoC10-001000 hi2; echo "exit $?" >> hi2_rococoC10-001000.log
./run1.sh ours fastxgemm-n2r6s0t2 symnorm ENOMOTO_MIP_HO3_SYMNORM=1
TL=120 ./run1.sh ours ns1830653 nbdiag120 ENOMOTO_MIP_NB_DIAG=1
TL=90 ENOMOTO_MIP_THREADS=1 ./run1.sh ours ns1830653 t1diag90 ENOMOTO_MIP_MEMBQ_DIAG=1 ENOMOTO_MIP_THREADS=1
touch q5.done
