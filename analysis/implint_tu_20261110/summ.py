"""PAPER7 の集計: 58 問 (b58*.jsonl、60 秒) と 47 問の一部 (b47*.jsonl、600 秒) を benchlib の形で比べ、全部の回で暫定解と下界の矛盾がないかを確かめる。
使い方: python summ.py > summ.txt"""
import glob, json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
sys.path.insert(0, str(D))
import benchlib as bl  # noqa: E402
from probs_all import P58, P47  # noqa: E402
AFF = {json.loads(l)["problem"]: json.loads(l) for l in open(D / "count.jsonl")}
aff = [p for p, r in AFF.items() if r.get("tu2")]
OLD = [p for p, r in AFF.items() if r.get("tu") and r["tu"] != 0]

def rows(pattern, tl):
    out = []
    for f in sorted(glob.glob(str(D / pattern))):
        if "partial" in f:
            continue
        for l in open(f):
            if l.strip():
                r = json.loads(l)
                r["rep"] = r["seed"] - 1
                out.append(r)
    return out

def show_table(R, probs, cfgs, tl):
    print(f"| 問題 | " + " | ".join(cfgs) + " |")
    print("|---|" + "---|" * len(cfgs))
    for p in probs:
        cells = []
        for c in cfgs:
            v = []
            for s in (0, 1):
                res = R.get((p, s), {}).get(c)
                if res is None:
                    v.append("-")
                elif bl.ok(res, tl):
                    v.append(f"{res['time']:.1f}")
                else:
                    b, o = res.get("bound"), res.get("objective")
                    g = abs(o - b) / max(abs(o), abs(b), 1e-9) if (b is not None and o is not None) else None
                    v.append(f"切れ({g:.1%})" if g is not None else f"{res.get('status')}")
            cells.append("・".join(v))
        print(f"| {p} | " + " | ".join(cells) + " |")

for name, pat, tl, probs in [("58 問 (60 秒)", "b58*.jsonl", 60.0, P58), ("47 問の一部 (600 秒)", "b47*.jsonl", 600.0, P47)]:
    rs = rows(pat, tl)
    if not rs:
        continue
    R = bl.table(rs)
    bl.TL = tl
    cfgs = sorted({r["cfg"] for r in rs})
    print(f"\n## {name}  設定 {cfgs}")
    keys_all = [k for k in R if k[0] in probs]
    if "def" in cfgs:
        for c in [c for c in cfgs if c != "def"]:
            ks = [k for k in keys_all if "def" in R[k] and c in R[k]]
            if not ks:
                continue
            bl.report(R, "def", c, ks, label=f"{name} 全部 def 対 {c}", show_root=False)
            ka = [k for k in ks if k[0] in aff]
            if ka and len(ka) != len(ks):
                bl.report(R, "def", c, ka, label=f"{name} 印が変わる問題だけ def 対 {c}", show_root=False)
    ps = [p for p in probs if p in aff and any((p, s) in R for s in (0, 1))]
    print(f"\n### 印が変わる問題の時間 (種 1・種 2、解けなければ最終ギャップ)\n")
    show_table(R, ps, cfgs, tl)

# 矛盾の検査: 問題ごとに、全部の回 (全部の設定・種・ファイル) の [min(暫定解, 下界), max(...)] に共通の点があるか
print("\n## 暫定解と下界の矛盾の検査 (全部の jsonl の全部の回)")
iv = defaultdict(list)
n = 0
for f in sorted(glob.glob(str(D / "b*.jsonl"))):
    for l in open(f):
        if not l.strip():
            continue
        r = json.loads(l); res = r["result"]; n += 1
        o, b = res.get("objective"), res.get("bound")
        if o is None or b is None or not (math.isfinite(o) and math.isfinite(b)):
            continue
        iv[r["problem"]].append((min(o, b), max(o, b), r["cfg"], r["seed"], Path(f).name))
bad = []
for p, L in iv.items():
    lo = max(L, key=lambda t: t[0]); hi = min(L, key=lambda t: t[1])
    tol = 1e-6 * max(1.0, abs(lo[0]), abs(hi[1]))
    if lo[0] > hi[1] + tol:
        bad.append((p, lo, hi))
print(f"回 {n}、問題 {len(iv)}、矛盾 {len(bad)}")
for p, lo, hi in bad:
    print("  矛盾", p, "下側の最大", lo, "上側の最小", hi)
