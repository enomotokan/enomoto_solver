#!/bin/bash
# 11728 の回と同じコード (inv/gfrsplit-20261021 にこのブランチの診断をマージした作業木 /home/user/enomoto_gfr、venv /home/user/venv_gfr) と
# 同じ設定 noF で、証明の刈り込みを LP で確かめる
cd "$(dirname "$0")"
export PY=/home/user/venv_gfr/bin/python
D="ENOMOTO_MIP_DEBUG_SOL=$PWD/mc11_pre.sol ENOMOTO_MIP_PROOF_DIAG=1 ENOMOTO_MIP_PROOF_VERIFY=1"
NOF="ENOMOTO_MIP_IMPLBD_GENINT=1 ENOMOTO_T_MIP_CUT_LP_RETRY=4 ENOMOTO_MIP_RESTART_REF=1"
./loop.sh gfr_noF mc11 ${N:-10} 4 60 $D $NOF
