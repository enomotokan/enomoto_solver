"""RCSTRONG の集計 (analysis/rcstrong_20261111.md): 58 問 (b58*.jsonl、60 秒) と 47 問の一部 (b47*.jsonl、600 秒) を benchlib の形で
既定と比べ、当たる問題 (RCSTRONG の行の数が 0 でない問題) を別に見る。診断の数の表と、全部の回で暫定解と下界の矛盾がないかも出す。
使い方: python summ.py > summ.txt"""
import glob, json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D.parent))
sys.path.insert(0, str(D.parent / "implint_tu_20261110"))
import benchlib as bl  # noqa: E402
from probs_all import P58, P47  # noqa: E402

PI = re.compile(r"implint (\d+)\+(\d+); root: in rc list (\d+), tightened without rounding (\d+), tighter by rounding (\d+), fixed by rounding (\d+); "
                r"nodes \(cumulative\): tighter by rounding (\d+), fixed by rounding (\d+), tightened without rounding (\d+)")
PK = re.compile(r"knapsack row \(all threads, cumulative\): calls (\d+), pruned (\d+), tightened (\d+), built (\d+) \(mean length (\S+)\)")


def rows(pattern):
    out = []
    for f in sorted(glob.glob(str(D / pattern))):
        for l in open(f):
            if l.strip():
                r = json.loads(l)
                r["rep"] = r["seed"] - 1
                out.append(r)
    return out


def counts(r):
    """1 回の RCSTRONG の行から、暗黙の整数の数 (最後の求解の行: 印の数, 根の表, 根で締めた, 切り捨てで締めた, 固定, ノードで切り捨てで締めた,
    ノードで固定) と、被約費用の行の数 (呼んだ, 刈った, 締めた, 平均の長さ) を返す。"""
    imp = None
    kn = None
    for l in r.get("rcs", []):
        m = PI.search(l)
        if m:
            g = [int(x) for x in m.groups()]
            # 再スタートで捨てた求解の行もあるので、印の数が最大の行を採る (根の数は列ごとに一度だけ数えたもの)
            v = (g[0] + g[1], g[2], g[3], g[4], g[5], g[6], g[7])
            if imp is None or v[0] > imp[0] or (v[0] == imp[0] and v[1] > imp[1]):
                imp = v
        m = PK.search(l)
        if m:
            kn = (int(m.group(1)), int(m.group(2)), int(m.group(3)), float(m.group(5)))
    return imp, kn


def cell(res, tl):
    if res is None:
        return "-"
    if bl.ok(res, tl):
        return f"{res['time']:.1f}"
    b, o = res.get("bound"), res.get("objective")
    g = abs(o - b) / max(abs(o), abs(b), 1e-9) if (b is not None and o is not None) else None
    return f"切れ({g:.1%})" if g is not None else f"{res.get('status')}"


# wheel ごとに分ける: b58.jsonl (wheel b: def・rcic・rck)、b58x.jsonl (wheel b: 揃って遅く見えた問題の種 3〜8)、
# b58r.jsonl (wheel c: r を加えた後の def・rcir)。def は同じ wheel のものとだけ比べる
for name, pat, tl, probs in [("58 問 (60 秒、wheel b)", "b58.jsonl", 60.0, P58),
                             ("揃って遅く見えた問題の種 3〜8 (60 秒、wheel b)", "b58x.jsonl", 60.0, P58),
                             ("58 問 (60 秒、wheel c)", "b58r.jsonl", 60.0, P58), ("47 問の一部 (600 秒)", "b47*.jsonl", 600.0, P47)]:
    rs = rows(pat)
    if not rs:
        continue
    R = bl.table(rs)
    for fn in (bl.ok, bl.t, bl.lt):
        fn.__defaults__ = (tl,)
    cfgs = sorted({r["cfg"] for r in rs}, key=lambda c: (c != "def", c))
    print(f"\n## {name}  設定 {cfgs}")
    # 当たる問題: 暗黙の整数で根かノードで締めた (i)、被約費用の行で締めたか刈った (k)
    hit_i, hit_k = set(), set()
    for r in rs:
        imp, kn = counts(r)
        if imp and (imp[3] or imp[5]):
            hit_i.add(r["problem"])
        if kn and (kn[1] or kn[2]):
            hit_k.add(r["problem"])
    keys_all = [k for k in R if k[0] in probs]
    for c in [c for c in cfgs if c != "def"]:
        ks = [k for k in keys_all if "def" in R[k] and c in R[k]]
        if not ks:
            continue
        bl.report(R, "def", c, ks, label=f"{name} 全部 def 対 {c}", show_root=False)
        hit = hit_k if "k" in c and "i" not in c else hit_i if "k" not in c else hit_i | hit_k
        ka = [k for k in ks if k[0] in hit]
        if ka and len(ka) != len(ks):
            bl.report(R, "def", c, ka, label=f"{name} 当たる問題だけ def 対 {c}", show_root=False)
    print("\n### 当たる問題の時間 (種の順、解けなければ最終ギャップ) と数 (最初の種)\n")
    print("暗黙の整数: 印 / 根の表に入った / 根で切り捨てで締めた / 根で固定 / ノードで切り捨てで締めた / ノードで固定。"
          "被約費用の行: 呼んだ / 刈った / 締めた / 長さ\n")
    print("| 問題 | " + " | ".join(cfgs) + " | 暗黙の整数 | 被約費用の行 |")
    print("|---|" + "---|" * (len(cfgs) + 2))
    for p in [p for p in probs if p in hit_i | hit_k]:
        reps = sorted({k[1] for k in R if k[0] == p})
        cells = ["・".join(cell(R.get((p, s), {}).get(c), tl) for s in reps) for c in cfgs]
        ci = ck = ""
        for c in cfgs:
            r1 = next((r for r in rs if r["problem"] == p and r["cfg"] == c), None)
            if r1 is None:
                continue
            imp, kn = counts(r1)
            if imp and not ci:
                ci = f"{imp[0]} / {imp[1]} / {imp[3]} / {imp[4]} / {imp[5]} / {imp[6]}"
            if kn and not ck:
                ck = f"{kn[0]} / {kn[1]} / {kn[2]} / {kn[3]:.0f}"
        print(f"| {p} | " + " | ".join(cells) + f" | {ci} | {ck} |")

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
