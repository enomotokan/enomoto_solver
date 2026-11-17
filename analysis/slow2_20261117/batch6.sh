#!/bin/bash
# 根の並列のサブ MIP の待ちの上限 (既存の試作 SUBCAP の J、下限を変える)
D=$(cd "$(dirname "$0")" && pwd)
"$D/one.sh" unitcal_7 ours 1 600 scJ ENOMOTO_MIP_SUBCAP=J
"$D/one.sh" unitcal_7 ours 1 600 scJf ENOMOTO_MIP_SUBCAP=J ENOMOTO_T_MIP_SUBCAP_FLOOR=0.02
"$D/one.sh" mzzv42z ours 1 600 scJf ENOMOTO_MIP_SUBCAP=J ENOMOTO_T_MIP_SUBCAP_FLOOR=0.02
