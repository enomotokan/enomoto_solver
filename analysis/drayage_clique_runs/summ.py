# side.sh の tsv を 問題 x 設定 の表にする (python summ.py <tsv>)
import sys, ast, collections
t = collections.defaultdict(list)
for line in open(sys.argv[1]):
    f = line.rstrip("\n").split("\t")
    if len(f) < 4 or not f[3].startswith("RESULT"): continue
    _, sec, d = f[3].split(" ", 2); d = eval(d, {"inf": float("inf"), "nan": float("nan")})
    obj = d.get("objective"); st = d.get("status")
    s = f"{sec}" if st.lower() == "optimal" else f"TL {obj if obj is None else round(obj,2)} lb {round(d.get('best_bound'),2)}"
    t[(f[1], f[2])].append(s)
for (p, c), v in sorted(t.items()): print(f"{p}\t{c}\t" + " | ".join(v))
