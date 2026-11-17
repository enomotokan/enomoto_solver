cd /home/user/enomoto_solver && source .venv/bin/activate
for p in supportcase40 tbfp-network CMS750_4 neos-3627168-kasai rococoC10-001000 unitcal_7; do
  python3 /tmp/claude-0/rdump/c16/run1.py CAND1116F.$p.k 1 600 '{"ENOMOTO_MIP_TC": "bsk"}' $p
done
