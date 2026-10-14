"""s4.jsonl (threads=4 既定) と n4.jsonl (threads=4、証明・衝突を共有しない) を比べ、証明・衝突の効き目を自前/取り込みで集計する。"""
import json
import math
import re
import sys

d = sys.argv[1]
TL = 60.0
load = lambda p: {r["name"]: r for r in map(json.loads, open(p))}
s4, n4 = load(f"{d}/s4.jsonl"), load(f"{d}/n4.jsonl")
t1 = load(f"{d}/t1.jsonl")
ok = lambda r: r.get("status") in ("optimal", "Optimal")
tm = lambda r: r["time"] if ok(r) else TL
PR = re.compile(r"proofs own/imp: live (\d+)/(\d+) eval (\d+)/(\d+) prune (\d+)/(\d+) tight (\d+)/(\d+) absorb (\d+)/(\d+); "
                r"lits own/imp: live (\d+)/(\d+) eval (\d+)/(\d+) prune (\d+)/(\d+) tight (\d+)/(\d+) aged (\d+)/(\d+) \| tid \d+ .* nodes (\d+)")


def agg(r):
    tot = [0] * 21
    for l in r.get("prof", [])[1:]:
        m = PR.search(l)
        if m:
            tot = [a + int(b) for a, b in zip(tot, m.groups())]
    return tot


def sgm(ts, s=10.0):
    return math.exp(sum(math.log(t + s) for t in ts) / len(ts)) - s


def gap(r):
    o, b = r.get("objective"), r.get("bound")
    if o is None or b is None or not math.isfinite(b):
        return None
    return abs(o - b) / max(abs(o), 1e-9) * 100


names = [n for n in s4 if n in n4]
f = lambda r: f"{r['time']:.2f}" if ok(r) else "t"
fg = lambda r: "" if ok(r) else ("-" if gap(r) is None else f"{gap(r):.2f}%")
print("| 問題 | 既定 [s] | 共有なし [s] | 既定 ギャップ | 共有なし ギャップ | 既定 ノード | 共有なし ノード | 証明 刈り 自前/取込 | 証明 締め 自前/取込 | 証明 差分更新 自前/取込 [万] | 証明 残り 自前/取込 | 衝突 刈り+締め 自前/取込 |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
T = [0] * 21
for n in names:
    a, b = s4[n], n4[n]
    g = agg(a)
    T = [x + y for x, y in zip(T, g)]
    print(f"| {n} | {f(a)} | {f(b)} | {fg(a)} | {fg(b)} | {a.get('nodes')} | {b.get('nodes')} | {g[4]}/{g[5]} | {g[6]}/{g[7]} | "
          f"{g[8]/1e4:.0f}/{g[9]/1e4:.0f} | {g[0]}/{g[1]} | {g[14]+g[16]}/{g[15]+g[17]} |")
print()
print(f"SGM: 既定 {sgm([tm(s4[n]) for n in names]):.2f} 共有なし {sgm([tm(n4[n]) for n in names]):.2f}"
      + (f" (参考 t1 {sgm([tm(t1[n]) for n in names if n in t1]):.2f})" if t1 else ""))
print(f"解けた: 既定 {sum(ok(s4[n]) for n in names)} 共有なし {sum(ok(n4[n]) for n in names)} / {len(names)}")
print(f"全体: 証明 刈り {T[4]}/{T[5]} 締め {T[6]}/{T[7]} 差分更新 {T[8]}/{T[9]} 調べた {T[2]}/{T[3]}; 衝突 刈り {T[14]}/{T[15]} 締め {T[16]}/{T[17]} 調べた {T[12]}/{T[13]}")
# 両方とも時間切れの問題のノード数・ギャップの比較
both_t = [n for n in names if not ok(s4[n]) and not ok(n4[n])]
nr = [n4[n]["nodes"] / max(1, s4[n]["nodes"]) for n in both_t if s4[n].get("nodes") and n4[n].get("nodes")]
print(f"両方時間切れ {len(both_t)} 問: ノード比 (共有なし/既定) 幾何平均 {math.exp(sum(map(math.log, nr))/len(nr)):.2f}")
gb = [(n, gap(s4[n]), gap(n4[n])) for n in both_t if gap(s4[n]) is not None and gap(n4[n]) is not None]
print("  ギャップが小さかった方: 既定 %d 問, 共有なし %d 問" % (sum(x < y - 1e-9 for _, x, y in gb), sum(y < x - 1e-9 for _, x, y in gb)))
