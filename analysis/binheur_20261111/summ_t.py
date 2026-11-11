"""res_t.jsonl の集計: 回ごとに、最良既知の値 (scripts/miplib2017-v31.solu) に届いた時刻 t_opt・最終の暫定解の時刻 t_last・
解き終わった時刻 T を並べ、次の型に分ける。
  易: 解けて T < 5 秒 / 証: 解けて t_opt <= T/2 (見つけるのは早く、残りは証明) / 遅: 解けて t_opt > T/2 (見つけるのが遅い) /
  切証: 時間切れで最良既知に届いている (証明が終わらない) / 切探: 時間切れで最良既知に届かない (主ギャップを表に出す)。
使い方: python summ_t.py > summ_t.txt"""
import json
from collections import Counter
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
SOLU = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="):
        SOLU[t[1]] = float(t[2])
K = json.loads((D / "kinds.json").read_text())
R = [json.loads(l) for l in (D / "res_t.jsonl").read_text().splitlines() if l.strip()]
def close(a, b): return a is not None and b is not None and abs(a - b) <= 1e-6 * max(1.0, abs(b))
def pgap(a, b):
    if a is None: return 1.0
    if close(a, b): return 0.0
    return 1.0 if a * b < 0 else abs(a - b) / max(abs(a), abs(b))
cnt = {}
print(f"{'問題':24s} {'秒':>4s} 種 {'0-1':>4s} {'型':4s} {'T':>7s} {'t_opt':>7s} {'t_last':>7s} {'最初':>7s} {'主ギャップ':>9s} 出どころ(最終) 行の形")
for r in R:
    p = r["problem"]; ref = SOLU.get(p); st = (r["status"] or "").lower()
    T = r["time"] if r["time"] is not None else r["tl"]
    incs = r["incs"]
    # ログの時刻は solve の中の時計。最良既知に届いた最初の更新
    t_opt = next((t for z, t, _ in incs if close(z, ref)), None)
    if t_opt is None and close(r["obj"], ref) and incs:
        t_opt = incs[-1][1]  # 磨き (polish) などで最後に届いた
    t_last = incs[-1][1] if incs else None
    ok = st == "optimal"
    if ok and T < 5: k = "易"
    elif ok: k = "証" if (t_opt if t_opt is not None else T) <= T / 2 else "遅"
    else: k = "切証" if close(r["obj"], ref) else "切探"
    pure = "純" if K[p]["pure01"] else "9割"
    cnt.setdefault((r["tl"], pure), Counter())[k] += 1
    rows = ",".join(f"{a}{b}" for a, b in sorted(K[p]["rows"].items(), key=lambda x: -x[1])[:3])
    f = lambda v: f"{v:7.1f}" if v is not None else "      -"
    print(f"{p:24s} {r['tl']:4d} {r['seed']:2d} {pure:>4s} {k:4s} {f(T)} {f(t_opt)} {f(t_last)} {f(r['first_t'])} {pgap(r['obj'], ref):9.2e} "
          f"{(incs[-1][2] if incs else '-')!s:12s} {rows}")
print()
for (tl, pure), c in sorted(cnt.items()):
    print(f"{tl} 秒 {pure}: " + " ".join(f"{k} {c[k]}" for k in ("易", "証", "遅", "切証", "切探")))
