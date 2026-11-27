"""run2.py の結果 (res2.jsonl) の集計: 8 問を既定 (def)・d・1 で比べる。下界の積分は summ.py と同じ定義 (d・1 は受け取った下界も入れる)。
暫定解の値と、primal の積分 (最適値との相対の差の時間の積分は記録に無いので、最後の暫定解の値だけ) も並べる。python summ2.py > summ2.txt"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res2.jsonl").read_text().splitlines()]
def integral(r):
    opt, T = r["opt"], r["tl"]
    res = r["res"]
    end = res.get("time", T) if res.get("status") == "optimal" else T
    ev = [(t, lb) for t, lb, _ in r["prog"] if math.isfinite(lb)]
    if r["cfg"] != "def": ev += [(t, v) for t, v in r["recv"]]
    ev.sort(); cur = -math.inf; tprev = 0.0; acc = 0.0
    def g(lb): return 1.0 if not math.isfinite(lb) else max(0.0, min(1.0, (opt - lb) / max(abs(opt), abs(lb), 1.0)))
    for t, lb in ev:
        t = min(t, end); acc += g(cur) * (t - tprev); tprev = t; cur = max(cur, lb)
    return acc + g(cur) * max(0.0, end - tprev)
by = {}
for r in rows: by.setdefault(r["problem"], {})[r["cfg"]] = r
print(f"TL {rows[0]['tl']} 秒、4 スレッド・種 1、ビルド 4a8431738。下界の積分は秒 (小さいほど良い)")
print("| 問題 | 最適値 | 既定: 下界 積分 | d: 下界 積分 (受け取り) | 1: 下界 積分 (受け取り) | 暫定解 既定 / d / 1 |")
print("|---|---:|---|---|---|---|")
rat = {"d": [], "1": []}
for p, d in by.items():
    if not all(c in d for c in ("def", "d", "1")): continue
    de = d["def"]; a = integral(de); cells = []
    for c in ("d", "1"):
        r = d[c]; b = integral(r); rat[c].append((p, a, b))
        cells.append(f"{r['res'].get('bound')} {b:.1f} ({len(r['recv'])})")
    print(f"| {p} | {de['opt']} | {de['res'].get('bound')} {a:.1f} | {cells[0]} | {cells[1]} | {de['res'].get('objective')} / {d['d']['res'].get('objective')} / {d['1']['res'].get('objective')} |")
for c, v in rat.items():
    gm = math.exp(sum(math.log((b + 1) / (a + 1)) for _, a, b in v) / len(v))
    print(f"\n{c}: 下界の積分の比 (/ 既定、shift 1 秒の幾何平均) {gm:.3f}、良くなった {[p for p, a, b in v if b < 0.95 * a]}、悪くなった {[p for p, a, b in v if b > 1.05 * a]}")
