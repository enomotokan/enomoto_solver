"""58 問の比べ (run58.py の jsonl): 設定ごとの解けた数・時間のずらした幾何平均 (解けない回は 60 秒、ずらし 10 秒)、問題ごとの比、
回をまたいだ食い違い (ある回の下界・最適値が別の回の暫定解より良い = 誤答の疑い) を出す。使い方: python summ58.py r58.jsonl 既定の設定 試作の設定"""
import json, math, sys
from collections import defaultdict
rows = [json.loads(l) for l in open(sys.argv[1]) if l.strip()]
base, proto = sys.argv[2], sys.argv[3]
T = 60.0
by = defaultdict(dict)
for r in rows:
    by[r["problem"]][(r["cfg"], r["rep"])] = r["result"]
def t_of(res):
    return res.get("time") if res.get("status") == "optimal" else T
def sgm(v, s=10.0):
    return math.exp(sum(math.log(x + s) for x in v) / len(v)) - s if v else float("nan")
reps = sorted({r["rep"] for r in rows})
pairs = [(p, rep) for p in by for rep in reps if (base, rep) in by[p] and (proto, rep) in by[p]]
tb = [min(t_of(by[p][(base, rep)]) or T, T) for p, rep in pairs]
tp = [min(t_of(by[p][(proto, rep)]) or T, T) for p, rep in pairs]
sb = sum(by[p][(base, rep)].get("status") == "optimal" for p, rep in pairs)
sp = sum(by[p][(proto, rep)].get("status") == "optimal" for p, rep in pairs)
print(f"組 {len(pairs)} (問題 {len({p for p, _ in pairs})}・回 {reps}): 解けた {base} {sb} / {proto} {sp}、SGM(10) {base} {sgm(tb):.2f} / {proto} {sgm(tp):.2f} (比 {(sgm(tp) + 10) / (sgm(tb) + 10):.3f})")
for rep in reps:
    pr = [(p, r) for p, r in pairs if r == rep]
    a = [min(t_of(by[p][(base, rep)]) or T, T) for p, _ in pr]; b = [min(t_of(by[p][(proto, rep)]) or T, T) for p, _ in pr]
    print(f"  rep {rep}: 組 {len(pr)} 解けた {sum(by[p][(base, rep)].get('status') == 'optimal' for p, _ in pr)} / {sum(by[p][(proto, rep)].get('status') == 'optimal' for p, _ in pr)}、SGM {sgm(a):.2f} / {sgm(b):.2f}")
print("\n問題ごと (時間の比 = (試作 + 10) / (既定 + 10) の回の幾何平均、|log 比| の大きい順):")
lst = []
for p in by:
    rs = [rep for rep in reps if (base, rep) in by[p] and (proto, rep) in by[p]]
    if not rs:
        continue
    ratio = math.exp(sum(math.log((min(t_of(by[p][(proto, r)]) or T, T) + 10) / (min(t_of(by[p][(base, r)]) or T, T) + 10)) for r in rs) / len(rs))
    fmt = lambda c: " ".join(f"{by[p][(c, r)].get('status', '?')[:4]} {by[p][(c, r)].get('time') or 0:.1f}s {by[p][(c, r)].get('nodes')}n" for r in rs)
    lst.append((abs(math.log(ratio)), p, ratio, fmt(base), fmt(proto)))
for _, p, ratio, a, b in sorted(lst, reverse=True):
    print(f"  {p}: 比 {ratio:.2f} | {base}: {a} | {proto}: {b}")
print("\n食い違いの疑い (向きは回の中の 下界 <= 暫定解 で判断):")
bad = 0
for p in by:
    res = list(by[p].items())
    sign = 1
    for _, r in res:
        if r.get("objective") is not None and r.get("bound") is not None and r["objective"] < r["bound"] - 1e-6 * max(1, abs(r["bound"])):
            sign = -1
    objs = [(k, r["objective"]) for k, r in res if r.get("objective") is not None]
    for k, r in res:
        b = r.get("bound")
        if b is None:
            continue
        for k2, o in objs:
            if sign * (b - o) > 1e-6 * max(1.0, abs(o)):
                print(f"  {p}: {k} の下界 {b} が {k2} の暫定解 {o} より良い"); bad += 1
print(f"  計 {bad} 件")
