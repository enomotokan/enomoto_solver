#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
until [ -e q9.done ]; do sleep 20; done
TL=90 ./run1.sh ours ns1830653 t1diag90 ENOMOTO_MIP_MEMBQ_DIAG=1 ENOMOTO_MIP_THREADS=1
touch q10.done
