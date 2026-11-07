"""run.py の jsonl を表にする: python summ.py file.jsonl"""
import json, sys
rows = {}
for l in open(sys.argv[1]):
    r = json.loads(l); rows.setdefault(r["problem"], []).append(r)
for p, rs in rows.items():
    for r in rs:
        res, i = r["result"], r["info"]
        print(f"{p:22s} {r['cfg']:4s} {r['rep']} {res.get('status','?'):10s} t={res.get('time',0):7.2f} nodes={res.get('nodes')} obj={res.get('objective')} bd={res.get('bound')} rootLP={i.get('root_lp')} cut={i.get('root_after_cuts')} added={i.get('cuts_added')} sub={i.get('subst')} rs={i.get('restarts')}")
        if len(sys.argv) > 2 and i.get("chosen"):
            print("      ", i["chosen"])
