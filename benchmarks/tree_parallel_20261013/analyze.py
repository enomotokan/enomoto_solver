"""t1.jsonl と t4.jsonl から、並列の効率と損失の内訳の表を作る。"""
import json
import math
import re
import sys

d = sys.argv[1]
TL = 60.0


def load(p):
    try:
        return {r["name"]: r for r in map(json.loads, open(p))}
    except FileNotFoundError:
        return {}


t1, t4 = load(f"{d}/t1.jsonl"), load(f"{d}/t4.jsonl")
ok = lambda r: r.get("status") in ("optimal", "Optimal")
tm = lambda r: r["time"] if ok(r) else TL


def head(r):
    m = re.search(r"total ([\d.]+)s tree start ([\d.]+)s workers start ([\d.]+)s", r["prof"][0]) if r.get("prof") else None
    return tuple(map(float, m.groups())) if m else (None, None, None)


ROW = re.compile(r"tid (\d+) ready ([\d.]+)s end ([\d.]+)s idle ([\d.]+)s \((\d+) waits\) tail ([\d.]+)s sync ([\d.]+)s nodes (\d+) \(lb>=opt (\d+)\) "
                 r"first LP \[dive (\d+) ([\d.]+)it ([\d.]+)ms \| queue (\d+) ([\d.]+)it ([\d.]+)ms \| pool (\d+) ([\d.]+)it ([\d.]+)ms\]")


def rows(r):
    out = []
    for l in r.get("prof", [])[1:]:
        m = ROW.search(l)
        if m:
            g = m.groups()
            out.append(dict(tid=int(g[0]), ready=float(g[1]), end=float(g[2]), idle=float(g[3]), waits=int(g[4]), tail=float(g[5]),
                            sync=float(g[6]), nodes=int(g[7]), waste=int(g[8]),
                            n=[int(g[9]), int(g[12]), int(g[15])], it=[float(g[10]), float(g[13]), float(g[16])],
                            ms=[float(g[11]), float(g[14]), float(g[17])]))
    return out


def sgm(ts, s=10.0):
    return math.exp(sum(math.log(t + s) for t in ts) / len(ts)) - s


names = [n for n in t1 if n in t4]
print(f"| 問題 | t1 [s] | t4 [s] | 速さ | t1 ノード | t4 ノード | ノード比 | 根 t1/t4 [s] | 木 t4 [s] | 待ち % | 終わりの待ち % | 従の準備 [s] | 同期 % | 最適値以上 t1/t4 % | 最初の LP 反復 (潜り/自分/共有) | 共有から |")
print("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|")
for n in names:
    a, b = t1[n], t4[n]
    h1, h4 = head(a), head(b)
    r1, r4 = rows(a), rows(b)
    if not r4 or h4[0] is None or max(r["end"] for r in r4) <= 0:
        print(f"| {n} | {a.get('time') and round(a['time'],2)} {a['status']} | {b.get('time') and round(b['time'],2)} {b['status']} | | {a.get('nodes')} | {b.get('nodes')} | | | | | | | | | | |")
        continue
    tree = max(r["end"] for r in r4)
    thr = len(r4)
    busy_tot = thr * tree
    idle = sum(r["idle"] for r in r4) + sum(r["ready"] for r in r4)
    tail = sum(r["tail"] for r in r4)
    sync = sum(r["sync"] for r in r4)
    ready = max(r["ready"] for r in r4)
    w1 = sum(r["waste"] for r in r1) / max(1, sum(r["nodes"] for r in r1)) * 100 if r1 else float("nan")
    w4 = sum(r["waste"] for r in r4) / max(1, sum(r["nodes"] for r in r4)) * 100
    k = [sum(r["n"][i] for r in r4) for i in range(3)]
    it = [sum(r["n"][i] * r["it"][i] for r in r4) / max(1, k[i]) for i in range(3)]
    sp = tm(a) / tm(b)
    nr = (b["nodes"] or 0) / max(1, a["nodes"] or 1)
    f = lambda r: f"{r['time']:.2f}" if ok(r) else "t"
    print(f"| {n} | {f(a)} | {f(b)} | {sp:.2f} | {a['nodes']} | {b['nodes']} | {nr:.2f} | {h1[1]:.1f}/{h4[1]:.1f} | {tree:.1f} | "
          f"{100*idle/busy_tot:.1f} | {100*tail/busy_tot:.1f} | {ready:.2f} | {100*sync/busy_tot:.2f} | {w1:.1f}/{w4:.1f} | "
          f"{it[0]:.0f}/{it[1]:.0f}/{it[2]:.0f} | {k[2]} |")
both = [n for n in names]
print()
print(f"SGM (shift 10, 未解決 {TL:g}s): t1 {sgm([tm(t1[n]) for n in both]):.2f}  t4 {sgm([tm(t4[n]) for n in both]):.2f}  "
      f"解けた t1 {sum(ok(t1[n]) for n in both)} t4 {sum(ok(t4[n]) for n in both)} / {len(both)}")
