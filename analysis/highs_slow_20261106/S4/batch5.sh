#!/bin/bash
# batch4 の後: S4 の試作 TINYCUT (neos-957323)・BGZERO (cod105)
cd "$(dirname "$0")"
while pgrep -f batch4.sh >/dev/null; do sleep 10; done
for s in 0 1 2; do python3 run1.py tc1_s$s neos-957323 ours $s ENOMOTO_MIP_S4_TINYCUT=1; done
for s in 0 1 2; do python3 run1.py nrbz_s$s cod105 ours $s ENOMOTO_MIP_NO_ROOT_RINS=1 ENOMOTO_MIP_S4_BGZERO=2; done
python3 run1.py bz_s0 cod105 ours 0 ENOMOTO_MIP_S4_BGZERO=2
