#!/bin/bash
# M で sw2 が 2 種とも遅かった neos-787933 (1.86・1.34) で SUBWAIT が働いているかのログ (種 1、600 秒、README 15 節)
cd "$(dirname "$0")"
./one.sh neos-787933 ours 1 600 b18sw2 ENOMOTO_T_MIP_SUBWAIT=2
