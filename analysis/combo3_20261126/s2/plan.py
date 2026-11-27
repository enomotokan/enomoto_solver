"""第 2 段の分担表を作る (analysis/combo_20261124/plan.py の写し): python plan.py [分担の数 (既定 11)] → shards.json。
- 240 問 × {1, 4 スレッド} × {def, c3} × 種 1、上限 600 秒 = 960 回。
- 同じ (問題, スレッド) の def と c3 は同じ分担で続けて回す (組)。組の中の順は組ごとに入れ替える (組の通し番号の偶奇)。
- 見積り: analysis/final_20261118 の既定の 1 回の実時間 (wall、待ち・読み込みを含む) を、def と c3 の両方に使う
  (1 スレッドは def_t1、4 スレッドは def。上限 600 秒の M の行。P の 60 秒の行は解けていればその wall)。行がなければ 630 秒 (上限 + 待ち)。
- 組を見積りの大きい順に、そのときいちばん軽い分担へ入れる (LPT)。"""
import glob, json, sys
from pathlib import Path
D = Path(__file__).resolve().parent
K = int(sys.argv[1]) if len(sys.argv) > 1 else 11
TL, SEED = 600, 1
PR = json.loads((D / "problems.json").read_text())["all"]
W = {}
for f in glob.glob(str(D.parents[1] / "final_20261118" / "res_*_s*.jsonl")):
    for l in open(f, encoding="utf-8"):
        if not l.strip():
            continue
        r = json.loads(l)
        th = {"def_t1": 1, "def": 4}.get(r["cfg"])
        st = str(r["result"].get("status") or "").lower()
        if th and (r.get("tl") == 600 or st in ("optimal", "infeasible")):
            if (r["problem"], th) not in W or r.get("tl") == 600:
                W[(r["problem"], th)] = r["wall"]
units = []
for i, (p, th) in enumerate((p, th) for p in PR for th in (1, 4)):
    w = W.get((p, th), TL + 30.0)
    cf = [f"def_t{th}", f"c3_t{th}"]
    if i % 2:
        cf.reverse()
    units.append((2 * w, [[p, c, SEED, TL] for c in cf]))
units.sort(key=lambda u: -u[0])
sh = [[] for _ in range(K)]; load = [0.0] * K
for w, runs in units:
    k = min(range(K), key=lambda j: load[j])
    sh[k] += runs; load[k] += w
known = sum(1 for p in PR for th in (1, 4) if (p, th) in W)
json.dump({"shards": sh, "est_hours": [round(x / 3600, 2) for x in load], "known_units": known}, open(D / "shards.json", "w"))
print(f"{K} 分担、{sum(len(s) for s in sh)} 回、見積りのある (問題, スレッド) {known} / {2 * len(PR)}")
print("分担ごとの見積り (時間):", [round(x / 3600, 2) for x in load], "計", round(sum(load) / 3600, 1))
