"""擬似費用の予測と実際の上がり幅の集計 (python pcana.py res_PC.jsonl)。`PCO` の行 (run_pc.py) から:
- 予測 = 観測の前の単位費用 × 予測の動く量 (親の値から境界まで)。実測 0 = 最初の LP の値 − 親の値、実測 F = 最後の LP の値 − 親の値
  (ノードの中でカット・伝播で解き直した後)。観測したノード (最初の LP が最適) だけ。
- 比 = log10((実測 + ε) / (予測 + ε))、ε = 1e-6 × max(1, |親の値|)。|比| ≤ log10 2 を「2 倍以内」。
- 時期: その回の最後の PCO の時刻を 3 等分 (始め・途中・終わり近く)。
- 模擬: 観測を時刻の順に並べ、(A) 今と同じ (最初の LP の上がり幅を学ぶ) と (B) 解き直した後の上がり幅を学ぶ、の 2 つの擬似費用で、
  次の観測の実測 F を予測した誤差を比べる (費用の式は pseudocost.rs の cost_up/down と同じ: 観測 n 回の平均と全体の平均を
  w = min(1, 0.9 + 0.1 n / 8) で混ぜ、n = 0 なら全体の平均)。スレッドの間の共有は無視 (全スレッドを 1 つの列として扱う)。"""
import gzip, json, math, statistics, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
L2 = math.log10(2)
def stats(v):
    if not v:
        return "     -"
    a = [abs(x) for x in v]
    return f"n {len(v):6d} 中央値|比| {statistics.median(a):.3f} 2倍以内 {sum(x <= L2 for x in a) / len(a):.2f} 10倍以上外れ {sum(x > 1 for x in a) / len(a):.2f} 偏り(中央値) {statistics.median(v):+.3f}"
out_rows = []
for l in open(D / sys.argv[1]):
    r = json.loads(l)
    if r.get("pclog") == "0":
        continue
    fn = D / "logs" / f"pc_{r['cfg']}_{r['problem']}_{r['tl']}_s{r['seed']}.pco.gz"
    rows = []
    for ln in gzip.open(fn, "rt"):
        f = ln.split()
        if len(f) < 20:
            continue
        rows.append(dict(tid=int(f[0]), n=int(f[1]), t=float(f[2]), d=int(f[3]), j=int(f[4]), up=f[5] == "1", pv=float(f[6]), po=float(f[7]),
                         dpred=float(f[8]), unit=float(f[9]), nobs=int(f[10]), obs=f[11] == "1", dact=float(f[12]), obj0=float(f[13]), obj=float(f[14]),
                         out=f[15], bin=f[16] == "1", cost=f[17] == "1", nnz=int(f[18]), ncut=int(f[19])))
    if not rows:
        continue
    T = max(x["t"] for x in rows)
    ob = [x for x in rows if x["obs"] and math.isfinite(x["obj0"])]
    def eps(x):
        return 1e-6 * max(1.0, abs(x["po"]))
    for x in ob:
        x["pred"] = x["unit"] * x["dpred"]
        x["a0"] = max(x["obj0"] - x["po"], 0.0)
        x["aF"] = max(x["obj"] - x["po"], 0.0) if math.isfinite(x["obj"]) else float("inf")
        e = eps(x)
        x["r0"] = math.log10((x["a0"] + e) / (x["pred"] + e))
        x["rF"] = math.log10((x["aF"] + e) / (x["pred"] + e)) if math.isfinite(x["aF"]) else None
        x["ph"] = min(2, int(3 * x["t"] / max(T, 1e-9)))
    print(f"==== {r['problem']} (種 {r['seed']}、{r['result'].get('status')} {r['result'].get('time', 0):.1f} 秒、ノード {r['result'].get('nodes')}、PCO {len(rows)} 行、観測 {len(ob)})")
    z0 = sum(x["a0"] <= eps(x) for x in ob) / max(len(ob), 1)
    print(f"  実測 0 (最初の LP で上がらない) の割合 {z0:.2f}、打ち切り・実行不能の子 {sum(1 for x in rows if not (x['obs'] and math.isfinite(x['obj0'])))}")
    res = [x for x in ob if x["aF"] > x["a0"] + eps(x)]
    print(f"  ノードの中の解き直しで上がった {len(res)} ({len(res) / max(len(ob), 1):.2f})、そのうち実測 F / 実測 0 の中央値 "
          f"{statistics.median([(x['aF'] + eps(x)) / (x['a0'] + eps(x)) for x in res if math.isfinite(x['aF'])]) if res else float('nan'):.2f}、F が inf (解き直しで刈れた) {sum(not math.isfinite(x['aF']) for x in ob)}")
    for ph in range(3):
        print(f"  時期 {ph}: 予測と実測 0 {stats([x['r0'] for x in ob if x['ph'] == ph])}")
    for lab, key in (("観測 0", lambda x: x["nobs"] == 0), ("観測 1-3", lambda x: 1 <= x["nobs"] <= 3), ("観測 4-7", lambda x: 4 <= x["nobs"] <= 7), ("観測 8+", lambda x: x["nobs"] >= 8),
                     ("二値", lambda x: x["bin"]), ("一般整数", lambda x: not x["bin"]), ("費用 0", lambda x: not x["cost"]), ("費用あり", lambda x: x["cost"]),
                     ("打ち切りあり", lambda x: x["ncut"] > 0), ("深さ < 10", lambda x: x["d"] < 10), ("深さ >= 30", lambda x: x["d"] >= 30)):
        print(f"  {lab:10s}: {stats([x['r0'] for x in ob if key(x)])}")
    # 模擬
    ob.sort(key=lambda x: (x["t"], x["n"]))
    errs = {"A": [], "B": []}
    for mode in ("A", "B"):
        s = defaultdict(float); k = defaultdict(int); tot = [0.0, 0.0]; totn = [0, 0]
        for x in ob:
            if not math.isfinite(x["aF"]) or x["dact"] <= 1e-9 or not math.isfinite(x["dact"]):
                continue
            key = (x["j"], x["up"]); u = int(x["up"])
            avg = tot[u] / totn[u] if totn[u] else 1.0
            n = k[key]
            c = avg if n == 0 else (lambda w: w * s[key] / n + (1 - w) * avg)(min(1.0, 0.9 + 0.1 * n / 8))
            e = eps(x)
            errs[mode].append(math.log10((x["aF"] + e) / (c * x["dpred"] + e)))
            g = x["a0"] if mode == "A" else x["aF"]
            unit = g / x["dact"]
            s[key] += unit; k[key] += 1; tot[u] += unit; totn[u] += 1
    print(f"  模擬 (実測 F を予測): A 最初の LP を学ぶ {stats(errs['A'])}")
    print(f"                         B 解き直しの後を学ぶ {stats(errs['B'])}")
    out_rows.append({"problem": r["problem"], "seed": r["seed"], "n_obs": len(ob), "zero0": z0, "resolved_up": len(res) / max(len(ob), 1),
                     "medA": statistics.median([abs(v) for v in errs["A"]]) if errs["A"] else None, "medB": statistics.median([abs(v) for v in errs["B"]]) if errs["B"] else None,
                     "med_ph": [statistics.median([abs(x["r0"]) for x in ob if x["ph"] == ph]) if any(x["ph"] == ph for x in ob) else None for ph in range(3)]})
json.dump(out_rows, open(D / "pc_rows.json", "w"), indent=0)
