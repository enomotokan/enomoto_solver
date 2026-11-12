"""rootsize2.jsonl の IMPLDIAG の行を集計する (analysis/foldpre_20261112.md): 分枝限定法が見る問題 (solver) の暗黙の整数の本数と刻みを
設定ごとに並べ、畳む前 (fold in) と畳んだ後 (fold out) で暗黙の整数・刻みが減る畳みを挙げる。"""
import json, re, sys
pat = re.compile(r"IMPLDIAG (.*?): n (\d+) m (\d+) ints (\d+) conts (\d+) \| implied primal (\d+) dual \+(\d+) ho2 \+(\d+) \| costed conts (\d+) \(not ho2 (\d+)\) \| obj step int (\S+) h (\S+) hf (\S+)")
R = {}
for l in open(sys.argv[1]):
    r = json.loads(l)
    d = {"solver": None, "in": [], "out": []}
    for ln in r["impl"]:
        m = pat.match(ln)
        if not m:
            continue
        g = m.groups()
        v = dict(n=int(g[1]), ints=int(g[3]), conts=int(g[4]), prim=int(g[5]), dual=int(g[6]), ho2=int(g[7]), cc=int(g[8]), ccn=int(g[9]), st=g[10], sth=g[11], sthf=g[12])
        if g[0] == "solver" and d["solver"] is None:
            d["solver"] = v
        elif g[0] == "fold in":
            d["in"].append(v)
        elif g[0] == "fold out":
            d["out"].append(v)
    R.setdefault(r["problem"], {})[r["cfg"]] = d
cf = ["nofold", "def", "fP", "fQ", "fB", "fBQ"]
f = lambda v: "-" if v is None else f"{v['prim']}+{v['dual']}+{v['ho2']} / {v['st']},{v['sth']},{v['sthf']}"
print("分枝限定法が見る問題: 暗黙の整数 (主側 + 双対側で増える + HO2 で増える) / 刻み (整数列だけ, HO2, HO2+固定)")
print("問題 | " + " | ".join(cf))
for p, d in R.items():
    print(p, "|", " | ".join(f(d.get(c, {}).get("solver")) for c in cf))
print()
print("畳む前 → 畳んだ後で、暗黙の整数 (主+双対+HO2) か刻みが減った畳み (設定、段)")
for p, d in R.items():
    for c in cf:
        x = d.get(c)
        if not x:
            continue
        for k, (a, b) in enumerate(zip(x["in"], x["out"])):
            ia, ib = a["prim"] + a["dual"] + a["ho2"], b["prim"] + b["dual"] + b["ho2"]
            lost_step = any(a[s] != "None" and b[s] == "None" for s in ("st", "sth", "sthf"))
            if ib < ia or lost_step:
                print(f"{p} {c} 段{k + 1}: 暗黙の整数 {a['prim']}+{a['dual']}+{a['ho2']} → {b['prim']}+{b['dual']}+{b['ho2']} (連続列 {a['conts']} → {b['conts']})、刻み {a['st']},{a['sth']},{a['sthf']} → {b['st']},{b['sth']},{b['sthf']}")
