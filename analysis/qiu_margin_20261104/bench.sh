#!/bin/bash
# bench1: 4 スレッド・60 秒・6 回ずつ。def (既定 m)、m0 (MEM3_TINY_MARGIN=0)、so (外側だけ小さい余裕: QIU_MARGIN_SITES=o:*)、ss (サブ MIP だけ: s:*)
cd /home/user/enomoto_solver; A=analysis/qiu_margin_20261104/run.sh
for r in 1 2 3 4 5 6; do
  $A b1_def_r$r 4 60
  $A b1_m0_r$r 4 60 ENOMOTO_MIP_MEM3_TINY_MARGIN=0
  $A b1_so_r$r 4 60 'ENOMOTO_MIP_QIU_MARGIN_SITES=o:*'
  $A b1_ss_r$r 4 60 'ENOMOTO_MIP_QIU_MARGIN_SITES=s:*'
done
