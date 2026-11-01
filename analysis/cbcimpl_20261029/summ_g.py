"""tab_gain の計測 (abg_m*.jsonl: Mittelmann 186 問 × 1 回、abg_s*.jsonl: 58 問 × 2 回) の集計 (2026-10-30)。
3 設定 (base, tab_nnz, tab_gain) がそろった (問題, 回) だけで比べる。時間切れ・異常終了は 60 秒"""
import json, math, sys
from pathlib import Path
HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from probs import LOSS  # noqa: E402
names = ["base", "tab_nnz", "tab_gain"]
def t(x): return min(x.get("time", 60), 60.0) if x.get("status") == "optimal" else 60.0
def ok(x): return x.get("status") == "optimal" and x.get("time", 99) < 60
def sgm(xs): return math.exp(sum(math.log(x + 10) for x in xs) / len(xs)) - 10
for label, pat in [("Mittelmann", "abg_m*.jsonl"), ("58 問", "abg_s*.jsonl")]:
    R, bad, objs = {}, [], {}
    for fn in sorted(HERE.glob(pat)):
        for l in open(fn, encoding="utf-8"):
            r = json.loads(l)
            res = r["result"]
            R.setdefault((r["problem"], r["rep"]), {})[r["cfg"]] = res
            if res.get("status") not in ("optimal", "time_limit"):
                bad.append((r["problem"], r["cfg"], res.get("status")))
            if res.get("status") == "optimal":
                objs.setdefault(r["problem"], set()).add(round(res["objective"], 4))
    keys = sorted(k for k, d in R.items() if all(c in d for c in names))
    print(f"== {label}: 3 設定がそろった (問題, 回) {len(keys)}")
    if not keys:
        continue
    b = sgm([t(R[k]["base"]) for k in keys])
    for c in names:
        s = sgm([t(R[k][c]) for k in keys])
        n = sum(ok(R[k][c]) for k in keys)
        gain = [k[0] for k in keys if ok(R[k][c]) and not ok(R[k]["base"])]
        lost = [k[0] for k in keys if not ok(R[k][c]) and ok(R[k]["base"])]
        print(f"{c:9s} SGM {s:6.2f} ({(s / b - 1) * 100:+.1f}%) 解けた {n:3d}  既定が解けず→解けた {len(gain)} {gain if c != 'base' else ''}  既定が解けた→解けず {len(lost)} {lost if c != 'base' else ''}")
    if label == "58 問":
        for grp, sel in [("負けている 21 問", lambda p: p in LOSS), ("それ以外の 37 問", lambda p: p not in LOSS)]:
            ks = [k for k in keys if sel(k[0])]
            if ks:
                b2 = sgm([t(R[k]["base"]) for k in ks])
                print(f"  {grp}: " + "  ".join(f"{c} {(sgm([t(R[k][c]) for k in ks]) / b2 - 1) * 100:+.1f}%" for c in names[1:]))
    print("最適値の食い違い:", [p for p, v in objs.items() if len(v) > 1], " 異常終了:", bad)
