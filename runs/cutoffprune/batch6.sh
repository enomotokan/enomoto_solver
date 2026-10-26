#!/bin/bash
# 証明の破れの分解 (作ったときにデバッグ解を切る証明の行・列) 付きで、打ち切り値 11728 を与えた mc11 を繰り返す
cd "$(dirname "$0")"
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
./loop.sh dc4c mc11 ${N:-10} 4 60 $D ENOMOTO_MIP_CUTOFF=11728
