#!/bin/bash
cd "$(dirname "$0")"
until grep -q ALL_DONE exp2.out; do sleep 3; done
r() { python3 run1.py ours "$@"; }
r neos-860300 cliqauto 1 300 ENOMOTO_MIP_CLIQUE_CUTS=auto
r neos-860300 cliqroot 1 300 ENOMOTO_MIP_CLIQUE_CUTS=root
r neos-860300 cliq1 1 300 ENOMOTO_MIP_CLIQUE_CUTS=1
r neos-860300 fbratio01 1 300 ENOMOTO_T_MIP_CLIQUE_FB_RATIO=0.1
echo ALL_DONE
