#!/bin/bash
cd $(dirname $0)
while pgrep -f "tree_close4_20261104/highs.sh\|./highs.sh" >/dev/null; do sleep 10; done
for p in sct2 unitcal_7 mushroom-best neos-1456979 traininstance6; do ./run1.sh def $p 600 ours ENOMOTO_MIP_TREE4_TRACE=5; done
