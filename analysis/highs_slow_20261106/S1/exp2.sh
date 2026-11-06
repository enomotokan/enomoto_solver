#!/bin/bash
cd "$(dirname "$0")"
r() { python3 run1.py ours "$@"; }
r physiciansched6-2 rinswide 1 300 ENOMOTO_MIP_S1_RINSWIDE=2
r hypothyroid-k1 ipmfind 1 300 ENOMOTO_MIP_S1_IPMFIND=1
r physiciansched6-2 rinswide 2 300 ENOMOTO_MIP_S1_RINSWIDE=2
r hypothyroid-k1 ipmfind 2 300 ENOMOTO_MIP_S1_IPMFIND=1
echo ALL_DONE
