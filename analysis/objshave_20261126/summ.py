"""run.py の結果 (res.jsonl) の集計: 問題ごとに、既定 (def) と OBJSHAVE (shave) の最後の下界・状態・時間と、下界の推移の積分 (dual integral) を並べる。
下界の推移 lb(t) は、進みの行の lb (その時刻までの最大) と、shave では受け取った下界 (received) の最大。最初の進みの行の前は下界なし。
ギャップ g(t) = min(1, (opt − lb(t)) / max(|opt|, |lb(t)|, 1))、下界なしは 1。dual integral = ∫_0^T g(t) dt (秒)。解けたら解けた時刻から後は 0。
opt は scripts/miplib2017-v31.solu の最適値。python summ.py > summ.txt"""
import json, math
from pathlib import Path
D = Path(__file__).resolve().parent
rows = [json.loads(l) for l in (D / "res.jsonl").read_text().splitlines()]
def events(r):
    ev = [(t, lb) for t, lb, _ in r["prog"] if math.isfinite(lb)]
    if r["cfg"] == "shave": ev += [(t, v) for t, v in r["recv"]]
    return sorted(ev)
def integral(r):
    opt, T = r["opt"], r["tl"]
    res = r["res"]
    end = res.get("time", T) if res.get("status") == "optimal" else T
    ev = events(r); cur = -math.inf; tprev = 0.0; acc = 0.0
    def g(lb): return 1.0 if not math.isfinite(lb) else max(0.0, min(1.0, (opt - lb) / max(abs(opt), abs(lb), 1.0)))
    for t, lb in ev:
        t = min(t, end)
        acc += g(cur) * (t - tprev); tprev = t; cur = max(cur, lb)
    acc += g(cur) * max(0.0, end - tprev)
    return acc
by = {}
for r in rows: by.setdefault(r["problem"], {})[r["cfg"]] = r
print(f"TL {rows[0]['tl'] if rows else '?'} 秒、4 スレッド・種 1。下界の積分は秒 (小さいほど良い)。働かない = 試作が働かなかった問題 (既定と同じ設定なので既定は回さない)")
print("| 問題 | 最適値 | 既定: 状態 下界 積分 | shave: 状態 下界 積分 | 証明の回数 |")
print("|---|---:|---|---|---|")
ratios = []
for p, d in by.items():
    sh = d.get("shave"); de = d.get("def")
    if sh is None: continue
    if not sh["active"]:
        print(f"| {p} | {sh['opt']} | (働かない) | {sh['res'].get('status')} {sh['res'].get('bound')} {integral(sh):.1f} | — |"); continue
    if de is None: continue
    a, b = integral(de), integral(sh)
    ratios.append((p, a, b))
    st = lambda r: f"{r['res'].get('status')} {r['res'].get('bound')} {integral(r):.1f}" + (f" ({r['res'].get('time'):.0f}s)" if r['res'].get('status') == 'optimal' else '')
    print(f"| {p} | {sh['opt']} | {st(de)} | {st(sh)} | {len(sh['recv'])} |")
if ratios:
    gm = math.exp(sum(math.log((b + 1) / (a + 1)) for _, a, b in ratios) / len(ratios))
    print(f"\n働いた {len(ratios)} 問の下界の積分の比 (shave / 既定、shift 1 秒の幾何平均): {gm:.3f}")
    print("良くなった:", sum(b < a * 0.95 for _, a, b in ratios), "悪くなった:", sum(b > a * 1.05 for _, a, b in ratios))
