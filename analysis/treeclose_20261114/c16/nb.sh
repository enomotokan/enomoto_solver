cd /home/user/enomoto_solver && source .venv/bin/activate
for p in supportcase40 mcsched CMS750_4 tbfp-network; do
  python3 /tmp/claude-0/rdump/c16/run1.py CAND1116.$p.nb 1 600 '{"ENOMOTO_MIP_NODESEL_BANDIT": "1"}' $p
  grep -a "NODESEL_BANDIT" /tmp/claude-0/rdump/c16/CAND1116.$p.nb.log
done
