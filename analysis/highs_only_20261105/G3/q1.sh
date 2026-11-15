#!/bin/bash
cd /home/user/enomoto_solver/analysis/highs_only_20261105/G3
./run1.sh ours net12 enum ENOMOTO_MIP_HO3_ENUM=1
for p in net12 ns1830653 rococoC10-001000 fastxgemm-n2r6s0t2; do ./run1.sh highs $p hi; done
for p in net12 ns1830653 rococoC10-001000 fastxgemm-n2r6s0t2; do ./run1.sh ours $p def; done
