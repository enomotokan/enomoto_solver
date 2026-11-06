#!/bin/bash
# batch5 の後: neos-957323 の木の弱さの切り分け (双対退化で強分枝を止めない既存の調整値)
cd "$(dirname "$0")"
while pgrep -f batch5.sh >/dev/null; do sleep 10; done
python3 run1.py tg1nds_s0 neos-957323 ours 0 ENOMOTO_MIP_S4_TINYGAP=1 ENOMOTO_MIP_SB_NO_DEGEN_SKIP=1
python3 run1.py nds_s0 neos-957323 ours 0 ENOMOTO_MIP_SB_NO_DEGEN_SKIP=1
