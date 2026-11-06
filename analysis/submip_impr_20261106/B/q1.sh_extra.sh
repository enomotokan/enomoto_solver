#!/bin/bash
# q1.sh の後 (q2.sh の 58 問の前。q2.sh は名前に q1.sh を含むこの台本の終わりも待つ): trento1 で、サブ MIP の orbital fixing を生成元 100 個までに絞った試作 (allof)
cd /home/user/enomoto_solver/analysis/submip_impr_20261106/B
while pgrep -f "q1\.sh$|q1\.sh " > /dev/null || pgrep -fx "/bin/bash ./q1.sh" > /dev/null; do sleep 30; done
D="ENOMOTO_MIP_SUBB_COUNT=1 ENOMOTO_MIP_MEMBQ_HDIAG=1"
ALL="ENOMOTO_MIP_SUBB_DIV=1 ENOMOTO_MIP_SUBB_POOL=1 ENOMOTO_MIP_SUBB_SYM=1"
./run1.sh allof1 trento1 $D $ALL ENOMOTO_T_MIP_SUBB_SYM_OF_MAX=100
