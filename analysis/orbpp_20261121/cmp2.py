"""新しいビルド (試作 ENOMOTO_MIP_ORBPP=cond を入れた版) での既定 (def2)・MIN_COLS=3 (mc32)・cond (cond2) の比べ。python cmp2.py > cmp2.txt
勝ち負けの決め方は cond.py の verdict と同じ (mc3・cond のそれぞれを既定と比べる)。cond の判断はログの「ORBPP cond:」の行から"""
import gzip, json, re
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
EPS = 1e-6


def verdict(a, b):
    x, y = a["result"], b["result"]
    oa, ob = x["status"] == "optimal", y["status"] == "optimal"
    if oa and ob:
        q = y["time"] / x["time"]
        return ("W" if q < 0.9 else "L" if q > 1.1 else "T"), f"秒 {x['time']:.0f}→{y['time']:.0f}"
    if oa != ob: return ("W" if ob else "L"), f"{x['status'][:4]}/{y['status'][:4]}"
    za, zb, ba, bb = x.get("objective"), y.get("objective"), x.get("bound"), y.get("bound")
    sc = max(1.0, abs(za or 1), abs(zb or 1))
    p = 0 if za is None or zb is None else (1 if zb < za - EPS * sc else -1 if zb > za + EPS * sc else 0)
    d = 0 if ba is None or bb is None else (1 if bb > ba + EPS * sc else -1 if bb < ba - EPS * sc else 0)
    s = f"主 {za:.6g}→{zb:.6g}, 双対 {ba:.6g}→{bb:.6g}"
    if p >= 0 and d >= 0 and (p or d): return "W", s
    if p <= 0 and d <= 0 and (p or d): return "L", s
    return "T", s


# (verdict は cond.py の写し)
R = defaultdict(dict)
for l in (D / "diag.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["tag"] in ("def2", "mc32", "cond2"): R[(r["problem"], r["threads"])][(r["tag"], r["seed"])] = r


def decision(p, s, th):
    f = D / "logs" / f"{p}_s{s}_t{th}_cond2.log.gz"
    if not f.exists(): return "?"
    m = re.search(r"ORBPP cond: (\d+) small orbitopes, static fixed (\d+), REPROBE fixed (\d+) .*-> (static|reverted to dynamic)", gzip.open(f, "rt").read())
    return f"{'静的' if m.group(4) == 'static' else '動的に戻す'} (固定 {m.group(3)})" if m else "対象なし"


def cell(r):
    x = r["result"]; return f"{x['status'][:4]} {x['time']:.0f} 秒 {x['nodes']} ノード {x.get('objective') or float('nan'):.6g}"


cnt = {"mc32": {"W": 0, "L": 0, "T": 0}, "cond2": {"W": 0, "L": 0, "T": 0}}
print("| 問題 | スレッド | 種 | cond の判断 | 既定 | mc3 | cond | mc3 対 既定 | cond 対 既定 |")
print("|---|---|---|---|---|---|---|---|---|")
for (p, th), d in R.items():
    for s in (1, 2):
        if not all((t, s) in d for t in ("def2", "mc32", "cond2")): continue
        a, b, c = d[("def2", s)], d[("mc32", s)], d[("cond2", s)]
        vb, vc = verdict(a, b), verdict(a, c)
        cnt["mc32"][vb[0]] += 1; cnt["cond2"][vc[0]] += 1
        print(f"| {p} | {th} | {s} | {decision(p, s, th)} | {cell(a)} | {cell(b)} | {cell(c)} | {vb[0]} | {vc[0]} |")
print()
for k, v in cnt.items(): print(f"{k}: 勝 {v['W']} / 負 {v['L']} / 分 {v['T']}")
