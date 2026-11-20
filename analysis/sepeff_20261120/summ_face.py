"""face.jsonl (FACECUT=1 の走査) と face_x.jsonl (比べ: 今の頂点でもう 1 回分離) の集計: python summ_face.py > summ_face.txt
- 退化の度合い: 根のカットのループを抜けた時点の w = 被約費用 0 の非基底の数 (最適面の次元の目安)、r = w / 固定でない非基底の数、分数の整数列 nfrac / nint
- 停滞: CUTSTOP の理由 (ENOMOTO_MIP_CUTSTOP_LOG) が stall 系 (stall・intobj_stall・highs_stall・stall_old・phase2_stall・progress_win・flat_stop)
- A: analysis/disj_20261119.md 8 節の「1 列の離接で上がる余地」(full モード)。小 = A < 0.1%、大 = A ≥ 3%
- FACECUT の伸び: (根のループの後の LP 値からの伸び) / (参照値 (scripts/miplib2017-v31.solu) − 根のループの後の LP 値)"""
import json, re, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
REF = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="):
        REF[t[1]] = float(t[2])
STALL = {"stall", "intobj_stall", "highs_stall", "stall_old", "phase2_stall", "progress_win", "flat_stop"}
A_SMALL = set("""eil33-2 bppc4-08 graph20-20-1rand markshare_4_0 misc07 neos-911970 noswot pk1 30n20b8 flugpl markshare2 mad gmu-35-40 gmu-35-50 glass4 fastxgemm-n2r6s0t2
ic97_potential mcsched neos-4338804-snowy traininstance2 traininstance6 supportcase18 supportcase33 ns1830653 rocI-4-11 rocII-5-11 sct2 csched008 wachplan bnatt400
ns1208400 CMS750_4 neos-5188808-nattai uccase12 brazil3 radiationm18-12-05 lectsched-5-obj chromaticindex512-7 chromaticindex1024-7 cod105 neos-2746589-doon
opm2-z10-s4 neos-2657525-crna graphdraw-domain neos-2978193-inde""".split())
A_LARGE = set(open(D / "targets_M.txt").read().split()) | {"binkar10_1", "neos-3083819-nubu", "khb05250"}
def load(name):
    p = D / f"{name}.jsonl"
    return {json.loads(l)["problem"]: json.loads(l) for l in open(p)} if p.exists() else {}
F, X = load("face"), load("face_x")
def q(v):
    if not v: return "-"
    v = sorted(v); n = len(v); f = lambda p: v[min(n - 1, int(p * (n - 1) + 0.5))]
    return f"{f(0.25):.3g} / {f(0.5):.3g} / {f(0.75):.3g} (n={n})"
rows = []
for p, r in F.items():
    fa, cs = r.get("face"), r.get("cutstop")
    if not fa or not cs: continue
    w, nb = fa["width"], max(fa["nonbasic"], 1)
    rows.append({"p": p, "set": r["set"], "reason": cs["reason"], "stall": cs["reason"] in STALL, "w": w, "r": w / nb, "fr": fa["nfrac"] / max(fa["nint"], 1), "nfrac": fa["nfrac"],
                 "A": "小" if p in A_SMALL else "大" if p in A_LARGE else "", "fc": r.get("facecut"), "fx": (X.get(p) or {}).get("facecut"), "obj0": cs["obj1"]})
print(f"問題 {len(rows)} (P {sum(x['set']=='P' for x in rows)}・M {sum(x['set']=='M' for x in rows)})")
from collections import Counter
print("ループを抜けた理由:", dict(Counter(x["reason"] for x in rows).most_common()))
print("\n## 1. 退化の度合い (25% / 50% / 75%)\n")
print("| 群 | w (被約費用 0 の非基底) | r = w / 非基底 | 分数の整数列の割合 |")
print("|---|---|---|---|")
for name, sel in [("停滞", lambda x: x["stall"]), ("停滞でない", lambda x: not x["stall"]), ("A 小 (< 0.1%)", lambda x: x["A"] == "小"), ("A 大 (≥ 3%)", lambda x: x["A"] == "大"),
                  ("停滞 かつ A 小", lambda x: x["stall"] and x["A"] == "小"), ("停滞 かつ A 大", lambda x: x["stall"] and x["A"] == "大")]:
    v = [x for x in rows if sel(x)]
    print(f"| {name} | {q([x['w'] for x in v])} | {q([x['r'] for x in v])} | {q([x['fr'] for x in v])} |")
def gfrac(x, fc):
    if not fc: return None
    g = REF.get(x["p"])
    if g is None: return None
    G = g - fc["obj0"]
    if abs(G) <= 1e-9 * max(1, abs(g)): return None
    return fc["gain"] / G
print("\n## 2. FACECUT の伸び (残りギャップに対する割合) と手間\n")
print("| 問題 | 集合 | A | 理由 | w | r | nfrac | FACECUT の伸び | 選んだ (x* を切る) | 手間 (1e9) | 比べ x の伸び | 比べの手間 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|")
T = set(open(D / "targets.txt").read().split())
for x in sorted([x for x in rows if x["p"] in T], key=lambda x: (x["set"], x["p"])):
    fc, fx = x["fc"], x["fx"]
    g1, g2 = gfrac(x, fc), gfrac(x, fx)
    s1 = "-" if g1 is None else f"{100*g1:.3f}%"
    s2 = "-" if g2 is None else f"{100*g2:.3f}%"
    print(f"| {x['p']} | {x['set']} | {x['A']} | {x['reason']} | {x['w']:.0f} | {x['r']:.2f} | {x['nfrac']} | {s1} | {fc['chosen'] if fc else '-'} ({fc['cut_x'] if fc else '-'}) | {fc['ticks']/1e9 if fc else 0:.3f} | {s2} | {fx['ticks']/1e9 if fx else 0:.3f} |")
for name, sel in [("全部", lambda x: True), ("停滞", lambda x: x["stall"]), ("対象 (停滞の P・A 大の M)", lambda x: x["p"] in T)]:
    v = [gfrac(x, x["fc"]) for x in rows if sel(x)]; v = [g for g in v if g is not None]
    vx = [gfrac(x, x["fx"]) for x in rows if sel(x)]; vx = [g for g in vx if g is not None]
    print(f"\n{name}: FACECUT の伸び (25/50/75%) {q([100*g for g in v])} %、> 0.1% の問題 {sum(g > 1e-3 for g in v)}、> 1% {sum(g > 1e-2 for g in v)}; 比べ x {q([100*g for g in vx])} %")
