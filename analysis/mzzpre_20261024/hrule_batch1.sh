#!/bin/bash
cd $(dirname $0)
M=../../.miplib_cache/mzzv42z.mps
for o in '' presolve_rule_off=32768 presolve_rule_off=65536 presolve_rule_off=98304 presolve=off mip_lifting_for_probing=0 mip_lifting_for_probing=1 mip_lifting_for_probing=2 presolve_rule_off=131072 presolve_rule_off=16384 presolve_rule_off=8192 presolve_rule_off=4096 presolve_rule_off=512 presolve_rule_off=64 presolve_rule_off=128 presolve_rule_off=262144; do
 python3 hrule_lp.py $M $o | tail -1
done
