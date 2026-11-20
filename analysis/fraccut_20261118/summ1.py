"""段 1 の集計: logs/<問題>_<設定>_s<種>.log から、外側の根のカットのループのラウンドの種類 (up・face・flat_tight) と、
そのうち下界がほぼ動かず (reldiff ≤ 1e-4) 非整数の列の数か度合いが 1 割以上減ったラウンドの数、根の下界・ノード数・時間を出す。"""
import json, re, sys, glob, math, collections
R = re.compile(r"FRACCUT round (\d+): dobj (\S+) reldiff (\S+) nfrac (\d+) -> (\d+) fsum (\S+) -> (\S+) active (\d+)/(\d+) prev_viol (\d+)/(\d+) maxviol \S+ (\w+)")
rows = {}
for f in sorted(glob.glob("logs/*.log")):
    m = re.match(r"logs/(.+)_(d|o|c)_s(\d+)\.log", f)
    if not m: continue
    p, c, s = m.groups()
    kinds = collections.Counter(); fdrop = 0; nr = 0; res = None
    for line in open(f, errors="replace"):
        if "FRACCUT round" in line:
            g = R.search(line)
            if not g: continue
            nr += 1
            k = g.group(12); kinds[k] += 1
            nf0, nf1, fs0, fs1 = int(g.group(4)), int(g.group(5)), float(g.group(6)), float(g.group(7))
            if k != "up" and (nf1 < 0.9 * nf0 or fs1 < 0.9 * fs0): fdrop += 1
        elif line.split("] ", 1)[-1].startswith("{"):
            try: res = json.loads(line.split("] ", 1)[1])
            except Exception: pass
    rows[(p, c, s)] = (nr, kinds, fdrop, res)
print(f"{'問題':22s} 設定 種 ラウンド up face flat 平らで非整数1割減 根の下界 ノード 時間 状態")
for (p, c, s), (nr, k, fd, r) in sorted(rows.items()):
    r = r or {}
    print(f"{p:22s} {c} {s} {nr:4d} {k['up']:3d} {k['face']:4d} {k['flat_tight']:4d} {fd:4d}  {r.get('root_bound', float('nan')):.6g} {r.get('nodes', -1):8d} {r.get('time', float('nan')):7.1f} {r.get('status')}")
# d に対する o・c の時間比 (時間切れは 120 秒)
def t(r): return r["time"] if r and r.get("status") == "optimal" else 120.0
for c in "oc":
    L = [math.log((t(rows[(p, c, s)][3]) + 1) / (t(rows[(p, "d", s)][3]) + 1)) for (p, cc, s) in rows if cc == c and (p, "d", s) in rows]
    if L: print(c, "/ d 時間の幾何平均比", round(math.exp(sum(L) / len(L)), 3), "組", len(L))
