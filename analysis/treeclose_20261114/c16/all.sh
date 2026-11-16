cd /home/user/enomoto_solver && source .venv/bin/activate
for p in CMS750_4 supportcase40 tbfp-network fiball; do for c in CAND1116 CAND1116F; do
  python3 /tmp/claude-0/rdump/c16/run1.py $c.$p 1 600 '{}' $p
done; done
