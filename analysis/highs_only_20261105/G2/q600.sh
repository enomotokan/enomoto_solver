#!/bin/bash
cd "$(dirname "$0")"
H="ENOMOTO_MIP_HO2_IMPLINT=1 ENOMOTO_MIP_HO2_GOBJ=1"
for p in csched008 swath3 neos-3627168-kasai sct2; do
  ./run1.sh def $p 600
  ./run1.sh ho2 $p 600 $H
  ./run1.sh ho2tr $p 600 $H ENOMOTO_MIP_TREE_RESTART=1 ENOMOTO_MIP_TREE_RESTART_LOG=1
done
