"""batch12 (60 秒、種 1・2、既定 p60def / VB_IMPL だけ p60vb / 選び方の変更だけ p60sel) の表を出す: python summ_p60.py"""
import glob, gzip, json, re, collections, math
R = collections.defaultdict(dict)
for f in glob.glob("logs/*_p60*_s*.log*"):
    m = re.match(r"logs/(.+)_p60(def|vb|sel)_s(\d)\.log", f)
    if not m: continue
    t = (gzip.open(f, "rt") if f.endswith(".gz") else open(f)).read().splitlines()
    try: r = json.loads(t[-1][t[-1].index("{"):])
    except Exception: continue
    tt = r["time"] if r["status"] in ("optimal", "infeasible") else 60.0
    R[(m[1], int(m[3]))][m[2]] = (tt, r["nodes"], r.get("root_bound"))
print("問題 種 | 既定 秒 (ノード) | vb | sel")
lv = collections.defaultdict(list)
for (p, s), d in sorted(R.items()):
    print(p, s, "|", " | ".join(f"{d[c][0]:.1f} ({d[c][1]})" if c in d else "-" for c in ("def", "vb", "sel")))
    for c in ("vb", "sel"):
        if "def" in d and c in d: lv[c].append(math.log((d[c][0] + 1) / (d["def"][0] + 1)))
for c, v in lv.items(): print(f"{c}/def SGM (shift 1): {math.exp(sum(v)/len(v))-1:+.1%} ({len(v)} 組)")
