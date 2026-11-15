cd /home/user/enomoto_solver/analysis/highs_only_20261105/G1
for p in fiball neos-933966 tr12-30 mushroom-best; do echo "$p def: $(./run1.sh def $p)"; done
echo "tr12-30 highs: $(./run1.sh highs tr12-30)"
echo ALLDONE
