#!/bin/bash
# PRESOLVE2: 対象の問題 × 既存の前処理の試作で、根の LP の前の大きさと根の LP を取る (epre.py)。1 つずつ順に回す。
cd "$(dirname "$0")"
P="${P:-neos859080 neos-3627168-kasai rococoC10-001000 neos-1445765 neos-3216931-puriri net12 mzzv42z mzzv11 physiciansched6-2}"
for p in $P; do
  python3 epre.py def $p
  python3 epre.py spl $p ENOMOTO_MIP_SPARSIFY_LATE=1
  python3 epre.py prr0 $p ENOMOTO_T_MIP_PRR_MIN_ROW_FRAC=0
  python3 epre.py int $p ENOMOTO_MIP_TREE2_INTAGG=1
  python3 epre.py pcl $p ENOMOTO_MIP_PROBE_CLIQUE=1
  python3 epre.py dom $p ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DUPCOL=1
  python3 epre.py enum $p ENOMOTO_MIP_HO3_ENUM=1
  python3 epre.py all $p ENOMOTO_MIP_SPARSIFY_LATE=1 ENOMOTO_T_MIP_PRR_MIN_ROW_FRAC=0 ENOMOTO_MIP_TREE2_INTAGG=1 ENOMOTO_MIP_PROBE_CLIQUE=1 ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DUPCOL=1 ENOMOTO_MIP_HO3_ENUM=1
done
