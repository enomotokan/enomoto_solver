R="ENOMOTO_MIP_DOMCOL=1 ENOMOTO_MIP_DOMCOL_PRED=1 ENOMOTO_MIP_DOMCOL_REV=1"
python ours.py fast0507 rev --log -- $R
for s in 2 3; do for p in rail507 fast0507; do
  python ours.py $p def --seed $s
  python ours.py $p rev --seed $s -- $R
done; done
