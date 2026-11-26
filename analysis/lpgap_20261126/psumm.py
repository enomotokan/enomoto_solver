"""P 1 スレッドの比べ (res_p_s0.jsonl) の集計: python psumm.py > psumm.txt
(問題, 種) の組ごとに def_t1 と ths_t1 の時間 (解けなければ 60 秒) の比を取り、ずらし幾何平均 (ずらし 1 秒) と、問題ごとのブートストラップの 95% 区間を出す。"""
import json, math, random, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = collections.defaultdict(dict)
for l in open(D / "res_p_s0.jsonl"):
    r = json.loads(l); x = r["result"]
    t = x.get("time", 60.0) if x.get("status") == "optimal" else 60.0
    R[(r["problem"], r["seed"])][r["cfg"]] = (x.get("status"), min(t, 60.0), x.get("nodes"), x.get("objective"))
pairs = {k: v for k, v in R.items() if "def_t1" in v and "ths_t1" in v}
lr = {k: math.log((v["ths_t1"][1] + 1) / (v["def_t1"][1] + 1)) for k, v in pairs.items()}
print(f"組 {len(pairs)} (問題 {len({k[0] for k in pairs})})")
for c in ("def_t1", "ths_t1"):
    print(c, "最適", sum(v[c][0] == "optimal" for v in pairs.values()), "ずらし幾何平均", round(math.exp(sum(math.log(v[c][1] + 1) for v in pairs.values()) / len(pairs)) - 1, 2))
g = math.exp(sum(lr.values()) / len(lr)); probs = sorted({k[0] for k in pairs})
bp = collections.defaultdict(list)
for k, v in lr.items(): bp[k[0]].append(v)
random.seed(1); bs = []
for _ in range(2000):
    s = [random.choice(probs) for _ in probs]; xs = [x for p in s for x in bp[p]]; bs.append(math.exp(sum(xs) / len(xs)))
bs.sort(); print(f"ths/def の比 {g:.3f} [{bs[50]:.3f}, {bs[1949]:.3f}]")
print("比が 1.2 倍を超えて動いた組 (問題 種 def 秒 → ths 秒、ノード):")
for k in sorted(pairs, key=lambda k: lr[k]):
    if abs(lr[k]) > math.log(1.2):
        d, t = pairs[k]["def_t1"], pairs[k]["ths_t1"]
        print(f"  {k[0]} {k[1]} {d[1]:.1f} → {t[1]:.1f} ({d[2]} → {t[2]})")
for k, v in pairs.items():
    a, b = v["def_t1"], v["ths_t1"]
    if a[0] == b[0] == "optimal" and a[3] is not None and b[3] is not None and abs(a[3] - b[3]) > 1e-6 * max(1, abs(a[3])):
        print("目的値の食い違い", k, a[3], b[3])
