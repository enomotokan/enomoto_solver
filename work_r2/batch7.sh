until grep -q RESULT fcsep2pool_2.out 2>/dev/null; do sleep 20; done
for rep in 1 2; do
./exp.sh tf10_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC=0.1
./exp.sh tf25_$rep 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_T_MIP_NODE_CUT_TIME_FRAC=0.25
done
