#!/bin/bash
# 11728 の型の再現: 証明による刈り込みを LP の解き直しで確かめる (ENOMOTO_MIP_PROOF_VERIFY) + デバッグ解
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
./loop.sh pv4 mc11 10 4 60 $D
./loop.sh pv4c mc11 6 4 60 $D ENOMOTO_MIP_CUTOFF=11728
./loop.sh pv1 mc11 3 1 60 $D
