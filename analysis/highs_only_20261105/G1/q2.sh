cd /home/user/enomoto_solver/analysis/highs_only_20261105/G1
D="ENOMOTO_MIP_MEMBQ_HDIAG=1 ENOMOTO_MIP_TREE4_TRACE=5"
SYM="ENOMOTO_T_MIP_PRESOLVE_WORK_SYM=1e10 ENOMOTO_T_MIP_PRESOLVE_WORK_SYM_LOG=1e11 ENOMOTO_T_MIP_SYM_TIME=20 ENOMOTO_T_MIP_SYM_TRIES=200000"
echo "fiball hd: $(./run1.sh hd fiball $D)"
echo "fiball grow: $(./run1.sh grow fiball $D ENOMOTO_MIP_MEMBQ_LNSGROW=1)"
echo "fiball bank: $(./run1.sh bank fiball $D ENOMOTO_MIP_HO1_LNSBANK=1)"
echo "fiball sym: $(./run1.sh sym fiball $D $SYM)"
echo "neos-933966 grow: $(./run1.sh grow neos-933966 $D ENOMOTO_MIP_MEMBQ_LNSGROW=1)"
echo "neos-933966 bank: $(./run1.sh bank neos-933966 $D ENOMOTO_MIP_HO1_LNSBANK=1)"
echo ALLDONE
