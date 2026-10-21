#!/bin/bash
# デバッグ解つきで、証明の誤りが頻繁に疑われる neos-911970 ほかを確かめる (断定できる判定 DEFINITE 付き)
cd "$(dirname "$0")"
V="ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
./loop.sh dv4 neos-911970 4 4 60 $V ENOMOTO_MIP_DEBUG_SOL=$PWD/neos-911970_pre.sol
./loop.sh dv4 rout 2 4 60 $V ENOMOTO_MIP_DEBUG_SOL=$PWD/rout_pre.sol
./loop.sh dv4 pk1 2 4 60 $V ENOMOTO_MIP_DEBUG_SOL=$PWD/pk1_pre.sol
./loop.sh dv4c mc11 4 4 60 $V ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_CUTOFF=11728
