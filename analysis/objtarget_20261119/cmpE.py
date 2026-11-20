"""res_traj_E_<設定>.jsonl を問題・種ごとに並べる: python cmpE.py 設定...  (60 秒の主のギャップ g と主の積分 PI、最初の解の木の時計の秒と値)"""
import json, glob, sys
from pathlib import Path
D = Path(__file__).resolve().parent
ref = {}
for l in open(D.parents[1] / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
def pg(o, r):
    if o is None: return 1.0
    if abs(o - r) <= 1e-9 * max(1, abs(r)): return 0.0
    if o * r < 0: return 1.0
    return abs(o - r) / max(abs(o), abs(r))
import os
PRE = os.environ.get("PRE", "E")
cf = sys.argv[1:]
R = {}
for c in cf:
    f = D / f"res_traj_{PRE}_{c}.jsonl"
    if f.exists():
        for l in open(f):
            r = json.loads(l); R[(r["problem"], r["seed"], c)] = r
def pint(r, z, tl=60):
    t0 = 0; g = 1.0; s = 0
    for t, v in r["inc"]:
        t = min(t, tl); s += g * (t - t0); t0 = t; g = pg(v, z)
    return s + g * (tl - t0)
for p in sorted({k[0] for k in R}):
    z = ref[p]
    for s in (1, 2):
        out = []
        for c in cf:
            r = R.get((p, s, c))
            if not r: out.append(f"{c} -"); continue
            inc = r["inc"]; fin = inc[-1][1] if inc else None
            res = r["result"]; tm = f" opt {res['time']:.1f}s" if res.get("status") == "optimal" else ""
            out.append(f"{c} {('%.4g' % fin) if fin is not None else 'none'} g{pg(fin, z):.3f} PI{pint(r, z):.1f}{tm}")
        print(f"{p} {s}: " + " | ".join(out))
