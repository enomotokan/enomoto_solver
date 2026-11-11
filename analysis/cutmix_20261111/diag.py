"""診断の表: 問題ごとの型の割合 (最初の根)・再スタートごとの 2 択の判定の切り替わり・分離器ごとの効き目 (SEPSTAT の根の伸びの割合)・
持ち上げつきの被覆 (lcstat) と zero-half (zhstat) の時間の比。
使い方: python diag.py OUT [基準の設定 stat] [比べる設定,...]  → 標準出力に markdown の表と相関"""
import json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
OUT = sys.argv[1]
BASE = sys.argv[2] if len(sys.argv) > 2 else "stat"
CMPS = (sys.argv[3] if len(sys.argv) > 3 else "lcstat,zhstat").split(",")
TL = 60.0
KEYS = ["vbrow", "vbcol", "cont", "genint", "knap", "knaptight", "mixknap", "setp", "introw", "intsmall", "flow"]
tp = re.compile(r"CUTMIX types restarts (\d+) m (\d+) n (\d+) (.*)")
sl = re.compile(r"SEPLEARN root restarts \d+ root (\S+)\s+calls \d+ cands \d+ chosen (\d+) gain (\S+)")


def parse_types(f):
    """再スタートごとの割合と判定 (同じ再スタートの 2 回目以降の行は最後のものを使う)"""
    per = {}
    for l in f.read_text().splitlines():
        m = tp.match(l)
        if not m:
            continue
        kv = m.group(4).split()
        d = {kv[i]: float(kv[i + 1]) for i in range(0, len(kv), 2)}
        d["m"], d["n"] = int(m.group(2)), int(m.group(3))
        per[int(m.group(1))] = d
    return per


def parse_gain(f):
    g = defaultdict(float)
    for l in f.read_text().splitlines():
        m = sl.match(l)
        if m:
            g[m.group(1)] += float(m.group(3))
    return g


res = defaultdict(dict)  # (問題, 設定) -> 種 -> 時間 (解けなければ TL)
for l in (D / f"{OUT}.jsonl").read_text().splitlines():
    if not l.strip():
        continue
    r = json.loads(l)
    st = r["result"].get("status")
    t = r["result"].get("time", TL) if st == "optimal" else TL
    res[(r["problem"], r["cfg"])][r["rep"]] = min(t, TL)
probs = sorted({p for p, _ in res})
types, flips, gains = {}, {}, {}
for p in probs:
    fs = sorted(D.glob(f"logs/{OUT}_{p}_{BASE}_s*.txt"))
    if not fs:
        continue
    per = parse_types(fs[0])
    if not per:
        continue
    types[p] = per[min(per)]
    nf = 0
    for f in fs:
        pr = parse_types(f)
        ks = sorted(pr)
        for a, b in zip(ks, ks[1:]):
            nf += (pr[a]["fc"] != pr[b]["fc"]) + (pr[a]["fcnode"] != pr[b]["fcnode"]) + ((pr[a]["genint"] > 0) != (pr[b]["genint"] > 0))
    flips[p] = (nf, max(len(parse_types(f)) for f in fs) - 1)
    g = defaultdict(float)
    for f in fs:
        for k, v in parse_gain(f).items():
            g[k] += v
    tot = sum(g.values())
    gains[p] = {k: v / tot for k, v in g.items()} if tot > 0 else {}


def ratio(p, c):
    a, b = res.get((p, BASE), {}), res.get((p, c), {})
    ks = sorted(set(a) & set(b))
    if not ks:
        return None
    return sum(math.log((b[k] + 10) / (a[k] + 10)) for k in ks) / len(ks)


print(f"| 問題 | 再スタート | 切替 | " + " | ".join(KEYS) + " | fc | fcnode | 主な分離器 (根の伸び) | " + " | ".join(CMPS) + " |")
print("|---|---|---|" + "---|" * len(KEYS) + "---|---|---|" + "---|" * len(CMPS))
for p in probs:
    if p not in types:
        continue
    t = types[p]
    top = sorted(gains[p].items(), key=lambda kv: -kv[1])[:3]
    tops = ", ".join(f"{k} {100 * v:.0f}%" for k, v in top if v >= 0.05)
    rs = [ratio(p, c) for c in CMPS]
    print(f"| {p} | {flips[p][1]} | {flips[p][0]} | " + " | ".join(f"{t[k]:.2f}" for k in KEYS) + f" | {int(t['fc'])} | {int(t['fcnode'])} | {tops} | "
          + " | ".join("" if r is None else f"{100 * (math.exp(r) - 1):+.0f}%" for r in rs) + " |")


def corr(xs, ys):
    n = len(xs)
    if n < 3:
        return float("nan")
    mx, my = sum(xs) / n, sum(ys) / n
    sx = math.sqrt(sum((x - mx) ** 2 for x in xs)); sy = math.sqrt(sum((y - my) ** 2 for y in ys))
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / (sx * sy) if sx > 0 and sy > 0 else float("nan")


def rank(v):
    o = sorted(range(len(v)), key=lambda i: v[i]); r = [0.0] * len(v)
    for k, i in enumerate(o):
        r[i] = k
    return r


print("\n相関 (Spearman、時間の比 log((t+10) の比) と割合、両方の時間が 1 秒未満の問題は除く)")
for c in CMPS:
    ps = [p for p in probs if p in types and ratio(p, c) is not None and max(max(res[(p, BASE)].values()), max(res[(p, c)].values())) >= 1.0]
    ys = [ratio(p, c) for p in ps]
    line = [f"{k} {corr(rank([types[p][k] for p in ps]), rank(ys)):+.2f}" for k in KEYS]
    print(f"- {c} ({len(ps)} 問): " + ", ".join(line))
print("\n相関 (Spearman、分離器の根の伸びの割合と型の割合)")
SRCS = ["rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "mcf", "modk"]
ps = [p for p in probs if p in types and gains.get(p)]
for s in SRCS:
    ys = [gains[p].get(s, 0.0) for p in ps]
    if sum(1 for y in ys if y > 0) < 3:
        continue
    line = [f"{k} {corr(rank([types[p][k] for p in ps]), rank(ys)):+.2f}" for k in KEYS]
    print(f"- {s} ({sum(1 for y in ys if y > 0)} 問で正): " + ", ".join(line))
