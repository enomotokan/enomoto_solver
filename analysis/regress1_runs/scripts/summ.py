import sys, json, re, glob
for f in sorted(sum([glob.glob(a) for a in sys.argv[1:]], [])):
    txt = open(f).read()
    m = re.search(r"RESULT (.*)", txt)
    r = json.loads(m.group(1)) if m else {}
    rs = "R" if re.search(r"[Rr]estart", txt) else "-"
    fs = r.get('first_solution_time'); fo = r.get('first_solution_objective')
    print(f"{f.split('/')[-1]:50s} {str(r.get('status')):12s} obj={r.get('objective')} bd={r.get('best_bound')} nodes={r.get('nodes')} first={fo}@{fs if fs is None else round(fs,1)} t={round(r.get('time',0),1)} {rs}")
