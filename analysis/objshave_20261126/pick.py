"""時間切れの問題から objective shaving・core の候補を選ぶ (gap.json の def_t1・def_t4、objform.jsonl、miplib2017-v31.solu)。
各問題で: 根の下界 r、最終の下界 b、最良の解 z、既知の最適値 opt (無ければ z)。
- 下界の進み = (b − r) / (opt − r) (根から最適までの差のうち木で縮めた割合)。
- 下界の残り = opt − b、整数の目的なら gcd で割った段数 (shaving で 1 段ずつ上げるときの回数)。
- 解の側の差 = z − opt (解が最適なら 0: 残りは下界だけの問題)。
python pick.py > pick.txt"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
gap = json.loads((WT / "analysis/gap240_20261126/gap.json").read_text())
form = {json.loads(l)["problem"]: json.loads(l) for l in (D / "objform.jsonl").read_text().splitlines()}
solu = {}
for l in (WT / "scripts/miplib2017-v31.solu").read_text().splitlines():
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): solu[t[1]] = (t[0], float(t[2]))
    elif len(t) >= 2 and t[0] == "=inf=": solu[t[1]] = ("=inf=", None)
rows = []
for g in gap:
    p = g["problem"]; f = form.get(p, {})
    for cfg in ("def_t1", "def_t4"):
        r = g.get(cfg)
        if not r or r.get("status") != "time_limit": continue
        z, b, rb = r.get("obj"), r.get("bound"), r.get("root_bound")
        k, opt = solu.get(p, (None, None))
        ref = opt if opt is not None else z
        if ref is None or b is None or rb is None or not all(map(math.isfinite, [ref, b, rb])):
            rows.append((p, cfg, None)); continue
        tot = ref - rb; prog = (b - rb) / tot if tot > 1e-9 else float("nan")
        rest = ref - b; sc = max(abs(ref), 1.0)
        steps = rest / f["gcd"] if f.get("obj_integral") and f.get("gcd") else None
        rows.append((p, cfg, dict(ref=ref, r=rb, b=b, z=z, opt=opt, kind=k, prog=prog, rest_rel=rest / sc, steps=steps, zgap=(z - ref) / sc if z is not None else None,
                                  others=max([x for x in ((g.get("scip") or {}).get("bound"), (g.get("highs") or {}).get("bound")) if x is not None and math.isfinite(x)], default=None), f=f)))
print("問題 | 設定 | 下界の進み | 下界の残り (相対) | 段数 | 解の差 (相対) | 他の最良の下界 − 当方 (相対) | 目的: 非零 (0-1/整数/連続) 整数値 gcd 種類")
for p, cfg, d in rows:
    if d is None: print(f"{p} | {cfg} | (数値なし)"); continue
    f = d["f"]; o = d["others"]
    ob = f"{(o - d['b']) / max(abs(d['ref']), 1):+.2e}" if o is not None else "—"
    st = f"{d['steps']:.0f}" if d["steps"] is not None else "—"
    print(f"{p} | {cfg} | {d['prog']:.3f} | {d['rest_rel']:.2e} | {st} | {d['zgap'] if d['zgap'] is None else format(d['zgap'],'.2e')} | {ob} | {f.get('obj_nnz')} ({f.get('obj_bin')}/{f.get('obj_int')}/{f.get('obj_cont')}) {'整' if f.get('obj_integral') else '実'} {f.get('gcd')} {f.get('distinct')}")
json.dump([(p, c, {k: v for k, v in d.items() if k != "f"} if d else None) for p, c, d in rows], open(D / "pick.json", "w"))
