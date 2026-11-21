"""対象 50 問 (targets.txt) で FACECUT・比べ x・MPSEP を並べる: python cmp_face.py > cmp_face.txt
- 閉じた割合 c = (根のループの後の LP 値 − 最初の LP 値) / (参照値 − 最初の LP 値)。MPSEP の効き = c(face、MPSEP 既定 oqmgs) − c(nompsep、MPSEP=0)
- FACECUT・x の伸び = 足した後の伸び / (参照値 − 根のループの後の LP 値) (残りギャップに対する割合)
- 手間: FACECUT・x は ticks (1e9)、MPSEP はループの秒 t_loop の差"""
import json, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
REF = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): REF[t[1]] = float(t[2])
L = lambda n: {json.loads(l)["problem"]: json.loads(l) for l in open(D / f"{n}.jsonl")}
F, X, N = L("face"), L("face_x"), L("nompsep")
def clos(r, p):
    cs = r.get("cutstop"); lp0 = r.get("lp0")
    if not cs or lp0 is None or p not in REF: return None
    G = REF[p] - lp0
    return None if abs(G) <= 1e-9 * max(1, abs(REF[p])) else (cs["obj1"] - lp0) / G
def gfr(r, p):
    fc = r.get("facecut")
    if not fc or p not in REF: return None
    G = REF[p] - fc["obj0"]
    return None if abs(G) <= 1e-9 * max(1, abs(REF[p])) else fc["gain"] / G
pc = lambda v: "-" if v is None else f"{100*v:.3f}%"
tk = lambda r: "-" if not r.get("facecut") else f"{r['facecut']['ticks']/1e9:.3f}"
print("| 問題 | 集合 | 閉じた割合 MPSEP 既定 / MPSEP=0 | MPSEP の効き | ループ秒 既定 / 0 | FACECUT の伸び (手間) | 比べ x の伸び (手間) | MPSEP=0 + FACECUT の伸び (手間) |")
print("|---|---|---|---|---|---|---|---|")
S = {"mp": [], "fc": [], "x": [], "nfc": [], "fct": [], "xt": [], "dt": []}
for p in sorted(open(D / "targets.txt").read().split(), key=lambda p: (F[p]["set"], p)):
    f, x, n = F[p], X.get(p, {}), N.get(p, {})
    c1, c0 = clos(f, p), clos(n, p)
    mp = None if c1 is None or c0 is None else c1 - c0
    t1 = (f.get("cutstop") or {}).get("t_loop"); t0 = (n.get("cutstop") or {}).get("t_loop")
    g1, gx, gn = gfr(f, p), gfr(x, p), gfr(n, p)
    for k, v in (("mp", mp), ("fc", g1), ("x", gx), ("nfc", gn)):
        if v is not None: S[k].append(v)
    if f.get("facecut"): S["fct"].append(f["facecut"]["ticks"] / 1e9)
    if x.get("facecut"): S["xt"].append(x["facecut"]["ticks"] / 1e9)
    if t1 is not None and t0 is not None: S["dt"].append(t1 - t0)
    print(f"| {p} | {f['set']} | {pc(c1)} / {pc(c0)} | {pc(mp)} | {t1} / {t0} | {pc(g1)} ({tk(f)}) | {pc(gx)} ({tk(x)}) | {pc(gn)} ({tk(n)}) |")
def q(v):
    if not v: return "-"
    v = sorted(v); n = len(v); f = lambda a: v[min(n - 1, int(a * (n - 1) + 0.5))]
    return f"{f(0.1):.3g} / {f(0.25):.3g} / {f(0.5):.3g} / {f(0.75):.3g} / {f(0.9):.3g} (n={n}、平均 {st.mean(v):.3g})"
print("\n10 / 25 / 50 / 75 / 90%")
for k, name in (("mp", "MPSEP の効き (閉じた割合の差)"), ("fc", "FACECUT の伸び"), ("x", "比べ x の伸び"), ("nfc", "MPSEP=0 + FACECUT の伸び"),
                ("fct", "FACECUT の手間 (1e9)"), ("xt", "比べ x の手間 (1e9)"), ("dt", "MPSEP のループ秒の差 (既定 − 0)")):
    print(f"- {name}: {q(S[k])}")
for k in ("mp", "fc", "x", "nfc"):
    print(f"- {k}: > 0.1% の問題 {sum(v > 1e-3 for v in S[k])}、> 1% {sum(v > 1e-2 for v in S[k])}、< −0.1% {sum(v < -1e-3 for v in S[k])}")
