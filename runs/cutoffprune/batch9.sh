#!/bin/bash
# 修正 (数値的に信用できない証明を捨てる) の確認: 決定的な誤りが出た条件 (打ち切り値 11728) と既定、ほかの問題の目的値
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1 ENOMOTO_MIP_PROOF_NUM_LOG=1"
./loop.sh fix4c mc11 ${N:-20} 4 60 $D ENOMOTO_MIP_CUTOFF=11728
./loop.sh fix4 mc11 6 4 60 $D
for p in beasleyC3 rout neos-911970 pk1; do ./loop.sh fix4 $p 3 4 60 ENOMOTO_MIP_PROOF_NUM_LOG=1; done
