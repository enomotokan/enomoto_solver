#!/bin/bash
cd $(dirname $0)
for p in sct2 unitcal_7 mushroom-best neos-1456979 traininstance6; do
  ./run1.sh def $p 600 ours
  ./run1.sh highs $p 600 highs
done
