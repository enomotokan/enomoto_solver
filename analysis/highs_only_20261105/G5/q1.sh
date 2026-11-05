#!/bin/bash
cd $(dirname $0)
until grep -q DONE /tmp/build.log; do sleep 10; done
./run1.sh highs mcsched 600 highs
./run1.sh highs neos-4722843-widden 600 highs
for p in traininstance6 mcsched neos-1456979 neos-4722843-widden; do ./run1.sh ours $p 600 def ENOMOTO_MIP_TREE4_TRACE=5; done
echo Q1DONE >> q.log
