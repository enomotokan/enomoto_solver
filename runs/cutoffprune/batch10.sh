#!/bin/bash
# 修正後に、誤った証明が出た条件 (打ち切り値 11728) を繰り返し、捨てられた証明 (reject numerically unreliable) が出るかを見る
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1 ENOMOTO_MIP_PROOF_NUM_LOG=1"
./loop.sh fixc2 mc11 ${N:-30} 4 60 $D ENOMOTO_MIP_CUTOFF=11728
