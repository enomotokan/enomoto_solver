cd /home/user/enomoto_solver/analysis/highs_only_20261105/G1
D="ENOMOTO_MIP_MEMBQ_HDIAG=1"
echo "neos-933966 re: $(./run1.sh re neos-933966 $D ENOMOTO_MIP_HO1_ROLEEARLY=1)"
echo "neos-933966 resc: $(./run1.sh resc neos-933966 $D ENOMOTO_MIP_HO1_ROLEEARLY=1 ENOMOTO_MIP_HO1_SUBCUT=1)"
echo "fiball resc: $(./run1.sh resc fiball $D ENOMOTO_MIP_HO1_ROLEEARLY=1 ENOMOTO_MIP_HO1_SUBCUT=1)"
echo ALLDONE
