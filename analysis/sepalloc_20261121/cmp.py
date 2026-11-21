"""根だけの比べ: python cmp.py 基準の名前 比べる名前 [...] (例 def as)。
- 閉じた割合 c = (最後の根のループの後の LP 値 − 最初の LP 値) / (参照値 − 最初の LP 値)。参照値は scripts/miplib2017-v31.solu
- 手間 tk = 外側の根のカットのループの手間 (CUTSTOP の tk_loop の和、診断の手間を含む)。根の LP の手間 tk_lp も出す
- 同じ手間での比べ: 基準の CUTTK (ラウンドの初めの LP 値と手間) の道のりで、比べるほうの手間までに基準が届いた値の閉じた割合 c@ (再スタートのない問題だけ)"""
import json, sys, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
REF = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): REF[t[1]] = float(t[2])
L = lambda n: {json.loads(l)["problem"]: json.loads(l) for l in open(D / f"{n}.jsonl")}
base, others = sys.argv[1], sys.argv[2:]
B = L(base); O = {n: L(n) for n in others}
def info(r):
    cs = r.get("cutstops") or []
    if not cs or r.get("lp0") is None: return None
    tk = sum(c.get("tk_loop", 0) for c in cs)
    return {"obj": cs[-1]["obj1"], "tk": tk, "rounds": sum(c["rounds"] for c in cs), "reason": cs[-1]["reason"], "restarts": len(cs) - 1, "tk_lp": cs[0].get("tk_lp")}
def clos(r, obj):
    p = r["problem"]; lp0 = r["lp0"]
    if p not in REF or obj is None: return None
    G = REF[p] - lp0
    if abs(G) <= 1e-9 * max(1.0, abs(REF[p])): return None
    return (obj - lp0) / G
def at(r, tk):
    """基準の道のりで手間 tk までに届いた LP 値 (ラウンドの初めの値。再スタートのない問題だけ)"""
    if len(r.get("cutstops") or []) != 1: return None
    v = None
    for _, o, t in r["cuttk"]:
        if t <= tk: v = o if v is None else max(v, o)
    fin = info(r)
    if fin and fin["tk"] <= tk: v = fin["obj"] if v is None else max(v, fin["obj"])
    return v
for n, OO in O.items():
    print(f"\n## {n} と {base}\n")
    print(f"| 問題 | 集合 | c {base} | c {n} | 差 | 手間 {base} / {n} (1e9) | c@ ({base} を {n} の手間で) | ラウンド | 理由 {base} / {n} |")
    print("|---|---|---|---|---|---|---|---|---|")
    dc, ratio, dat, nb, nw, rows = [], [], [], 0, 0, []
    for p in sorted(set(B) & set(OO), key=lambda p: (B[p]["set"], p)):
        a, b = info(B[p]), info(OO[p])
        if not a or not b: continue
        ca, cb = clos(B[p], a["obj"]), clos(OO[p], b["obj"])
        cat = clos(B[p], at(B[p], b["tk"]))
        if ca is not None and cb is not None:
            dc.append(cb - ca)
            if cb - ca > 1e-3: nb += 1
            if cb - ca < -1e-3: nw += 1
        if a["tk"] > 0 and b["tk"] > 0: ratio.append(b["tk"] / a["tk"])
        if cat is not None and cb is not None: dat.append(cb - cat)
        pc = lambda v: "-" if v is None else f"{100*v:.2f}%"
        if ca is None or cb is None or abs(cb - ca) > 1e-3 or abs(b["tk"] / max(a["tk"], 1) - 1) > 0.2:
            rows.append(f"| {p} | {B[p]['set']} | {pc(ca)} | {pc(cb)} | {pc(None if ca is None or cb is None else cb - ca)} | {a['tk']/1e9:.2f} / {b['tk']/1e9:.2f} | {pc(cat)} | {a['rounds']} / {b['rounds']} | {a['reason']} / {b['reason']} |")
    print("\n".join(rows))
    q = lambda v: "-" if not v else (lambda s: f"{s[int(0.1*(len(s)-1)+.5)]:.3g} / {s[int(0.25*(len(s)-1)+.5)]:.3g} / {s[int(0.5*(len(s)-1)+.5)]:.3g} / {s[int(0.75*(len(s)-1)+.5)]:.3g} / {s[int(0.9*(len(s)-1)+.5)]:.3g} (n={len(s)}、平均 {st.mean(s):.3g})")(sorted(v))
    gm = lambda v: __import__("math").exp(st.mean(__import__("math").log(x) for x in v)) if v else float("nan")
    print(f"\n- 閉じた割合の差 ({n} − {base}) 10/25/50/75/90%: {q(dc)}。0.1% 超で良い {nb} 問・悪い {nw} 問")
    print(f"- 手間の比 ({n} / {base}): {q(ratio)}、幾何平均 {gm(ratio):.3f}")
    print(f"- 同じ手間での差 (c {n} − c@ {base}): {q(dat)}。良い {sum(v > 1e-3 for v in dat)} 問・悪い {sum(v < -1e-3 for v in dat)} 問")
    print(f"- 表は閉じた割合が 0.1% 超変わるか手間が 2 割超変わった問題だけ ({len(rows)} 問)")
