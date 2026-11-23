#!/bin/sh
# 列挙の前処理の置く場所 (ENOMOTO_MIP_ENUM_WHERE) の比べ: 7 問 × 9 通り (既定を含む)、根だけ・1 スレッド、1 本ずつ順に。リポジトリの根から sh analysis/cutgap_20261123/enumwhere.sh
for p in net12 istanbul-no-cutoff ns1830653 mcsched seymour1 swath1 tbfp-network; do
  for c in "ew_def ENOMOTO_MIP_LOG=1" "ew_all_after ENOMOTO_MIP_ENUM_PRESOLVE=all" "ew_all_before ENOMOTO_MIP_ENUM_PRESOLVE=all ENOMOTO_MIP_ENUM_WHERE=before" "ew_all_loop ENOMOTO_MIP_ENUM_PRESOLVE=all ENOMOTO_MIP_ENUM_WHERE=loop" \
           "ew_impl_after ENOMOTO_MIP_ENUM_PRESOLVE=allimpl" "ew_impl_before ENOMOTO_MIP_ENUM_PRESOLVE=allimpl ENOMOTO_MIP_ENUM_WHERE=before" "ew_impl_loop ENOMOTO_MIP_ENUM_PRESOLVE=allimpl ENOMOTO_MIP_ENUM_WHERE=loop" \
           "ew_all_loop_prr ENOMOTO_MIP_ENUM_PRESOLVE=all ENOMOTO_MIP_ENUM_WHERE=loop ENOMOTO_MIP_PRR_TIGHT=1" "ew_impl_loop_prr ENOMOTO_MIP_ENUM_PRESOLVE=allimpl ENOMOTO_MIP_ENUM_WHERE=loop ENOMOTO_MIP_PRR_TIGHT=1"; do
    set -- $c; tag=$1; shift
    python3 analysis/cutgap_20261123/ours_root.py $p $tag "$@" > /dev/null
  done
  echo "$p done $(date +%T)"
done
