"""gainstall の集計: python summ.py [設定...] > summ.txt (2 回目は python summ.py 2 [設定...] > summ2.txt)。res_root.jsonl (根だけ) と res_full.jsonl (120 秒の通し) から、問題ごとに
外側の最初の根のカットのループの止めた理由・ラウンド数・根の下界 (根だけの回) と、通しの状態・時間を並べる。"""
import json, math, sys
CF = sys.argv[1:] or ["def", "g1e-3", "g1e-2", "g3e-2"]
def load(f):
    R = {}
    try:
        for l in open(f):
            r = json.loads(l); R[(r["problem"], r["cfg"])] = r
    except FileNotFoundError:
        pass
    return R
SUF = "2" if CF and CF[0] == "2" else ""
if SUF: CF = CF[1:] or ["def", "g3e-2", "w1e-2", "w3e-2"]
RR, RF = load(f"res_root{SUF}.jsonl"), load(f"res_full{SUF}.jsonl")
probs = list(dict.fromkeys(p for p, _ in list(RR) + list(RF)))
print("問題 | " + " | ".join(f"{c}: 理由 ラウンド 根の下界 / 通し" for c in CF))
T = {c: [] for c in CF}; S = {c: 0 for c in CF}
for p in probs:
    cells = []
    for c in CF:
        r, f = RR.get((p, c)), RF.get((p, c))
        cs = (r or {}).get("cutstop_first") or {}
        rb = (r or {}).get("result", {}).get("root_bound")
        fr = (f or {}).get("result", {})
        ok = str(fr.get("status", "")).lower() in ("optimal", "infeasible")
        t = fr.get("time") or 120
        if f: T[c].append(min(t, 120) if ok else 120); S[c] += ok
        cells.append(f"{cs.get('reason', '-')} {cs.get('rounds', '-')} {rb if rb is None else f'{rb:.6g}'} / {fr.get('status', '-')[:4]} {t:.1f}")
    print(p, "|", " | ".join(cells))
g = lambda xs: math.exp(sum(math.log(x + 1) for x in xs) / len(xs)) - 1 if xs else float("nan")
print()
for c in CF:
    print(f"{c}: 通しで解けた {S[c]} / {len(T[c])}、SGM (ずらし 1 秒、時間切れは 120 秒) {g(T[c]):.2f}、def 比 {(g(T[c]) + 1) / (g(T['def']) + 1):.3f}")
