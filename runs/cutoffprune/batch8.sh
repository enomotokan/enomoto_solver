#!/bin/bash
# 決定的な誤り (作った時点でデバッグ解を切る証明) が出た条件 (打ち切り値 11728) を、破れの分解の診断付きで繰り返す
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
./loop.sh dc4c2 mc11 ${N:-30} 4 60 $D ENOMOTO_MIP_CUTOFF=11728
