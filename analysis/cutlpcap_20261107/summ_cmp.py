"""rootcmp*.jsonl の根の比べを表にする (問題 x 設定: 終わり方・ループの後の値・時刻、a の解き直しの回数)。使い方: python summ_cmp.py rootcmp.jsonl ..."""
import json, sys
from collections import defaultdict
T = defaultdict(dict); cfgs = []
for f in sys.argv[1:]:
    for l in open(f):
        r = json.loads(l); c = r.get("cutloop") or {}
        if r["cfg"] not in cfgs: cfgs.append(r["cfg"])
        rd = c.get("rounds", {})
        nok = len([k for k in rd if "obj" in rd[k]])
        lastt = max([v.get("t", 0) for v in rd.values()], default=0)
        T[r["problem"]][r["cfg"]] = f"{c.get('end')} {(c.get('obj1') if c.get('obj1') is not None else float('nan')):.7g} r{nok} {lastt:.0f}s" + (f" a{r.get('cl_a')}" if r.get("cl_a") else "") + (f" n{r.get('cl_n')}" if r.get("cl_n") else "")
print("| 問題 | " + " | ".join(cfgs) + " |"); print("|---" * (len(cfgs) + 1) + "|")
for p in T:
    print(f"| {p} | " + " | ".join(T[p].get(c, "") for c in cfgs) + " |")
