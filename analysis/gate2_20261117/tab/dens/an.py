"""密度で割る点数の根だけの試し (feat.jsonl の札 d_*) を並べる: 根の終わりの下界・根の時間・根の tableau の分離の時間の合計・
最後の「cut loop done」のカットの本数と LP の中の本数・プールの非零。python dens/an.py"""
import json, gzip, re, os
D = os.path.dirname(os.path.abspath(__file__)) + "/.."
F = {}
for l in open(D + "/feat.jsonl"):
    r = json.loads(l)
    if r["tag"].startswith("d_"): F[(r["problem"], r["tag"][2:])] = r
tags = ["def", "tab", "a0.5", "a1", "b0.5", "b1", "c0.5", "c1"]
pat = re.compile(r"SEP tableau ([\d.]+)s .*light false submip false")
def info(p, t):
    r = F.get((p, t))
    if not r: return None
    ts = sum(float(m.group(1)) for x in r["tab"] for m in [pat.search(x)] if m)
    loop = None
    f = f"{D}/logs/feat_{p}_d_{t}.log.gz"
    if os.path.exists(f):
        for l in gzip.open(f, "rt"):
            m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP, pool \d+ cuts \((\d+) nonzeros\)", l)
            if m and loop is None: loop = m.groups()
    return r.get("root_bound"), r.get("time"), ts, loop, r.get("status")
probs = ["qnet1", "10teams", "neos-911970", "mc11", "nw04", "30n20b8", "roll3000", "neos-3627168-kasai", "fiball"]
for p in probs:
    print(p)
    for t in tags:
        x = info(p, t)
        if x: print(f"  {t:5s} 根の下界 {x[0]!s:>22.22s}  時間 {x[1]:6.1f}  tableau {x[2]:6.2f}s  カット(足した/LP/プール非零) {x[3]}  {x[4]}")
