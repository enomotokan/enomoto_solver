#!/bin/bash
cd $(dirname $0)
until grep -q Q2DONE q.log; do sleep 20; done
./run1.sh ours mcsched 600 hcm ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_HIGHS_CUTMGMT=1
./run1.sh ours neos-4722843-widden 600 robps ENOMOTO_MIP_TREE4_TRACE=5 ENOMOTO_MIP_CUT_LP_CAP_ROBUST=ps
echo Q3DONE >> q.log
