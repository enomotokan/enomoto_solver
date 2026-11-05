cd /home/user/enomoto_solver/analysis/highs_only_20261105/G1
D="ENOMOTO_MIP_MEMBQ_HDIAG=1 ENOMOTO_MIP_TREE4_TRACE=5"
echo "neos-933966 sc: $(./run1.sh sc neos-933966 $D ENOMOTO_MIP_HO1_SUBCUT=1)"
echo "fiball sc: $(./run1.sh sc fiball $D ENOMOTO_MIP_HO1_SUBCUT=1)"
echo "tr12-30 sc: $(./run1.sh sc tr12-30 $D ENOMOTO_MIP_HO1_SUBCUT=1)"
echo "mushroom-best sc: $(./run1.sh sc mushroom-best $D ENOMOTO_MIP_HO1_SUBCUT=1)"
echo ALLDONE
