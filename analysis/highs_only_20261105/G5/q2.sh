#!/bin/bash
cd $(dirname $0)
until grep -q Q1DONE q.log; do sleep 20; done
./run1.sh ours neos-1456979 60 sep60 ENOMOTO_MIP_DEBUG_SEPPROF=1
./run1.sh ours neos-1456979 60 hstall60 ENOMOTO_MIP_DEBUG_SEPPROF=1 ENOMOTO_MIP_CUT_STALL_HIGHS=1
./run1.sh ours traininstance6 600 prr ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_PROBE_REPRESOLVE=1
./run1.sh ours neos-4722843-widden 600 intagg ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_TREE2_INTAGG=1
echo Q2DONE >> q.log
