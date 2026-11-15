#!/bin/bash
# q1.sh の後 (q2.sh の 58 問の前。q2.sh は名前に q1.sh を含むこの台本の終わりも待つ):
# trento1 で、サブ MIP の orbital fixing を生成元 100 個までに絞った試作 (allof)、広げる量を直した版 (allv2 = DIV=2 + POOL + SYM) を 3 問
cd /home/user/enomoto_solver/analysis/submip_impr_20261106/B
while pgrep -fx "/bin/bash ./q1.sh" > /dev/null; do sleep 30; done
D="ENOMOTO_MIP_SUBB_COUNT=1 ENOMOTO_MIP_MEMBQ_HDIAG=1"
ALL="ENOMOTO_MIP_SUBB_DIV=1 ENOMOTO_MIP_SUBB_POOL=1 ENOMOTO_MIP_SUBB_SYM=1"
ALL2="ENOMOTO_MIP_SUBB_DIV=2 ENOMOTO_MIP_SUBB_POOL=1 ENOMOTO_MIP_SUBB_SYM=1"
./run1.sh allof1 trento1 $D $ALL ENOMOTO_T_MIP_SUBB_SYM_OF_MAX=100
for p in nexp-150-20-8-5 fiball neos-933966; do ./run1.sh allv2_1 $p $D $ALL2; done
