#!/bin/bash
# neos-1456979 (sw2 が全種 2.37 倍遅く、SUBWAIT が根の被約費用のサブ MIP を 9.2 秒・ノード 0 で止めている) の def のログ (種 1・2、600 秒、README 16 節)
cd "$(dirname "$0")"
while pgrep -f swprobe.py >/dev/null; do sleep 20; done
./one.sh neos-1456979 ours 1 600 b19def
./one.sh neos-1456979 ours 1 600 b19sw2 ENOMOTO_T_MIP_SUBWAIT=2
