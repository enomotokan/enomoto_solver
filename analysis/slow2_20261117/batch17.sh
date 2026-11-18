#!/bin/bash
# M で sw2 が遅かった CMS750_4・pg5_34 (各 1.30) で SUBWAIT が働いているかのログ (種 1、600 秒、README 15 節)
cd "$(dirname "$0")"
./one.sh CMS750_4 ours 1 600 b17sw2 ENOMOTO_T_MIP_SUBWAIT=2
./one.sh pg5_34 ours 1 600 b17sw2 ENOMOTO_T_MIP_SUBWAIT=2
