until grep -q "def s2" highs2.log 2>/dev/null; do sleep 20; done
H="ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_NODE_CUTS=1 ENOMOTO_MIP_CUT_AGE_HIGHS=1 ENOMOTO_T_MIP_CUT_AGE_PER_ROW=0"
for rep in 1 2; do
./exp.sh nc_ah30_$rep 300 4 $H ENOMOTO_T_MIP_CUT_AGE_LIMIT=30
./exp.sh nc_ah100_$rep 300 4 $H ENOMOTO_T_MIP_CUT_AGE_LIMIT=100
./exp.sh nodecuts_$((rep+2)) 300 4 ENOMOTO_MIP_TREE_LOG=1 ENOMOTO_MIP_NODE_CUTS=1
done
