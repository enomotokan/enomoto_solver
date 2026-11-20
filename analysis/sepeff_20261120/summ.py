"""root.jsonl の集計 (python summ.py > summ.txt)。カットの種類ごとに、根の下界の伸びの按分 (gain) と分離の手間 (ticks) から
- 取り分 share = gain_k / Σgain (その問題の根のカットの伸びのうちの割合)
- 手間の割合 tshare = ticks_k / Σticks (分離の手間のうちの割合)
- 相対の効き目 rel = share / tshare (1 より大きければ、手間の割合より多く下界を上げた)
- 手間あたりの伸び eff = (gain_k / max(|根の LP|, 1)) / (ticks_k / 1e9) (根の LP の値で割った伸び / 手間の秒)
を問題ごとに出し、種類ごとの分布 (四分位) と、zero-half の対象の問題の値を並べる。手間が 1e6 未満 (≒ 1 ミリ秒) の種類は数えない。"""
import json, math, statistics as st
from pathlib import Path
D = Path(__file__).resolve().parent
R = [json.loads(l) for l in open(D / "root.jsonl")]
def q(v):
    if not v: return "-"
    v = sorted(v); n = len(v)
    f = lambda p: v[min(n - 1, int(p * (n - 1) + 0.5))]
    return f"{f(0.1):.3g} / {f(0.25):.3g} / {f(0.5):.3g} / {f(0.75):.3g} / {f(0.9):.3g}"
rows = {}
for r in R:
    K = r["kinds"]; G = sum(max(e["gain"], 0) for e in K.values()); T = sum(e["ticks"] for e in K.values())
    sc = max(abs(r["lp0"] or 0), 1.0)
    for k, e in K.items():
        if e["ticks"] < 1e6: continue
        rows.setdefault(k, []).append({"p": r["problem"], "set": r["set"], "share": e["gain"] / G if G > 0 else 0.0, "tshare": e["ticks"] / T if T > 0 else 0.0,
                                       "eff": e["gain"] / sc / (e["ticks"] / 1e9), "chosen": e["chosen"], "act1": e["act1"], "cands": e["cands"], "G": G, "ticks": e["ticks"], "gain": e["gain"]})
print(f"問題 {len(R)} (P {sum(r['set']=='P' for r in R)}・M {sum(r['set']=='M' for r in R)})、根のカットで伸びた問題 {sum(1 for r in R if sum(max(e['gain'],0) for e in r['kinds'].values()) > 0)}")
print("\n## 種類ごとの分布 (手間が 1e6 以上の問題。値は 10% / 25% / 50% / 75% / 90%)\n")
print("| 種類 | 問題 | 伸び > 0 | 取り分 share | 手間の割合 tshare | 相対の効き目 rel (伸び > 0 の問題) | 手間あたりの伸び eff |")
print("|---|---|---|---|---|---|---|")
for k, v in sorted(rows.items(), key=lambda kv: -len(kv[1])):
    pos = [x for x in v if x["share"] > 0]
    print(f"| {k} | {len(v)} | {len(pos)} | {q([x['share'] for x in v])} | {q([x['tshare'] for x in v])} | {q([x['share'] / x['tshare'] for x in pos if x['tshare'] > 0])} | {q([x['eff'] for x in v])} |")
FOCUS = {"seymour": "効く", "nw04": "手間だけ", "misc07": "手間だけ", "eil33-2": "手間だけ", "graph20-20-1rand": "手間だけ"}
print("\n## zero-half: 問題ごと (rel の大きい順)\n")
print("| 問題 | 集合 | 狙い | 候補 | 入れた | 直後に効いた | 取り分 | 手間の割合 | rel | eff |")
print("|---|---|---|---|---|---|---|---|---|---|")
zh = sorted(rows.get("zerohalf", []), key=lambda x: -(x["share"] / x["tshare"] if x["tshare"] > 0 else 0))
for x in zh:
    rel = x["share"] / x["tshare"] if x["tshare"] > 0 else 0
    print(f"| {x['p']} | {x['set']} | {FOCUS.get(x['p'], '')} | {x['cands']} | {x['chosen']} | {x['act1']} | {x['share']:.3f} | {x['tshare']:.3f} | {rel:.2f} | {x['eff']:.3g} |")
missing = [p for p in FOCUS if p not in {x['p'] for x in zh}]
if missing: print("\nzero-half の手間が 1e6 未満か結果なし:", missing)
