"""threads/bench.jsonl の集計 (スレッド数 1・2・8・16 と、4 スレッド = bench20 の設定 p の回 34・35)。
解けた = status optimal かつ 60 秒以内 (summ8.py と同じ)。SGM はずらし 10 秒、解けない回は 60 秒。
使い方: python summ.py [bench の名前 (コンマ区切り、既定 bench)]。門ありの 4 スレッドは python summ.py bench,gate"""
import json, math, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
names = sys.argv[1].split(",") if len(sys.argv) > 1 else ["bench"]
R = defaultdict(dict)   # 問題 -> 設定 -> 結果
X = defaultdict(dict)   # 問題 -> 設定 -> 行全体 (壁時計・スレッド・RSS)
for l in (l for n in names for l in open(D / f"{n}.jsonl")):
    r = json.loads(l)
    R[r["problem"]][r["cfg"]] = r["result"]; X[r["problem"]][r["cfg"]] = r
for f in (WT / "analysis" / "halve_submip_20261030").glob("bench20_s*.jsonl"):
    for l in open(f):
        r = json.loads(l)
        if r["cfg"] == "p" and r["rep"] in (34, 35) and r["problem"] in PROBS:
            R[r["problem"]][f"t4r{r['rep']}"] = r["result"]
H = {}
for l in open(WT / "analysis" / "highs58_20261030" / "highs.jsonl"):
    r = json.loads(l); H[r["problem"]] = r["result"]
def ok(res): return res.get("status") == "optimal" and res.get("time", 99) <= 60.0
def t(res): return res["time"] if ok(res) else 60.0
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
CF = [c for c in ["t1", "t2", "t4", "t4r34", "t4r35", "t4g", "t8", "t16", "t8r", "t12"] if any(c in R[p] for p in PROBS)]
print("| 設定 | 問題数 | 解けた | SGM (秒) | crash | 60 秒超え (61 秒超) | 最大スレッド数 (中央値/最大) | 最大 RSS MB (中央値/最大) |")
print("|---|---|---|---|---|---|---|---|")
for c in CF:
    ps = [p for p in PROBS if c in R[p]]
    if not ps:
        continue
    crash = [p for p in ps if R[p][c].get("status") not in ("optimal", "time_limit", "infeasible")]
    over = [p for p in ps if R[p][c].get("time", 0) > 61 or X[p].get(c, {}).get("wall", 0) > 70]
    ths = sorted(X[p][c]["max_threads"] for p in ps if c in X[p]); rss = sorted(X[p][c]["max_rss_kb"] // 1024 for p in ps if c in X[p])
    med = lambda v: f"{v[len(v) // 2]}/{v[-1]}" if v else "-"
    print(f"| {c} | {len(ps)} | {sum(ok(R[p][c]) for p in ps)} | {sgm([t(R[p][c]) for p in ps]):.2f} | {len(crash)} {crash if crash else ''} | {len(over)} {over if over else ''} | {med(ths)} | {med(rss)} |")
# 4 スレッドの 2 回と揃えた SGM (全設定が揃っている問題だけ)
full = [p for p in PROBS if all(c in R[p] for c in CF)]
print(f"\n全設定が揃っている {len(full)} 問の SGM: " + "、".join(f"{c} {sgm([t(R[p][c]) for p in full]):.2f}" for c in CF))
print("\n片方だけ解けた・2 倍以上 (5 秒以上) 違う問題 (TL = 解けない):")
print("| 問題 | " + " | ".join(CF) + " | HiGHS |")
print("|---|" + "---|" * (len(CF) + 1))
for p in PROBS:
    d = R[p]; ts = [t(d[c]) for c in CF if c in d]; oks = {ok(d[c]) for c in CF if c in d}
    if len(oks) > 1 or (max(ts) >= 5 and max(ts) >= 2 * min(ts)):
        f = lambda r: (f"{r['time']:.1f}" if ok(r) else "TL") if r else "-"
        h = H.get(p, {}); hs = f"{h['time']:.1f}" if h.get("status") == "Optimal" else "TL"
        print(f"| {p} | " + " | ".join(f(d.get(c)) for c in CF) + f" | {hs} |")
print("\n異常 (optimal・time_limit 以外) と食い違い (optimal 同士、または HiGHS の Optimal と相対 1e-6 を超える差):")
for p in PROBS:
    d = R[p]
    for c, v in d.items():
        if v.get("status") not in ("optimal", "time_limit"):
            print("  ", p, c, v.get("status"), round(v.get("time", 0), 1), str(v.get("error", ""))[:120])
    zs = {c: d[c]["objective"] for c in d if d[c].get("status") == "optimal"}
    h = H.get(p, {})
    if h.get("status") == "Optimal":
        zs["HiGHS"] = h["objective"]
    if zs:
        v = list(zs.values())
        if max(v) - min(v) > 1e-6 * max(1, abs(v[0])):
            print("   食い違い", p, zs)
    # 時間切れの暫定解が HiGHS の最適値より良い (下界側に入る) なら誤答
    if h.get("status") == "Optimal":
        for c, r in d.items():
            if r.get("status") == "time_limit" and r.get("objective") is not None and r["objective"] < h["objective"] - 1e-6 * max(1, abs(h["objective"])):
                print("   暫定解が HiGHS の最適値より良い (最小化として)", p, c, r["objective"], h["objective"])
            if r.get("bound") is not None and r["bound"] > h["objective"] + 1e-6 * max(1, abs(h["objective"])):
                print("   下界が HiGHS の最適値を超える", p, c, r["bound"], h["objective"])
