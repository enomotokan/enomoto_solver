T="ENOMOTO_MIP_TREE2_FORCEQ=1 ENOMOTO_MIP_TREE2_INTAGG=1"
python run.py r600 def 600 neos-1445765
for p in rococoC10-001000 mcsched neos-4722843-widden; do
  python run.py r600 trial 600 $p $T
  python run.py r600 def 600 $p
done
python run.py r600 trialcq 600 rococoC10-001000 $T ENOMOTO_MIP_CLIQUE_CUTS=on
P=$(cd ../.. && python3 -c "import sys; sys.path.insert(0,'analysis/cbcimpl_20261029'); import probs; print(' '.join(probs.PROBS))")
for p in $P; do
  python run.py b58 def 60 $p
  python run.py b58 trial 60 $p $T
done
echo ALLDONE
