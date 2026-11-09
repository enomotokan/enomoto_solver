"""bench.py の結果を問題 x 設定で並べる。使い方: python summ.py OUT.jsonl [...]
列: 時間 (時間切れは TL)・根の下界 (root_bound)・ノード・根のループの終わりの時刻 (最初の `Root cuts` の秒)・
ノードの LP の平均の行・反復/LP・ノードの LP の秒・zh trial の g_z/gap と判定"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = []
for f in sys.argv[1:]:
    for l in Path(f).read_text().splitlines():
        if l.strip():
            r = json.loads(l); r["stem"] = Path(f).stem; rows.append(r)
def logi(r):
    t = (D / "logs" / f"{r['stem']}_{r['problem']}_{r['cfg']}_s{r['rep']}.txt")
    s = t.read_text() if t.exists() else ""
    d = {}
    m = re.search(r"Root cuts: objective (\S+) -> (\S+), (\d+) cuts in the LP \(([\d.]+)s\)", s)
    if m:
        d["rc_t"] = float(m.group(4)); d["rc_obj"] = float(m.group(2)); d["rc_ncuts"] = int(m.group(3))
    m = re.search(r"node diag: nodes (\d+) node LPs (\d+) mean rows ([\d.]+) iters (\d+) \(([\d.]+)/LP\) LP secs ([\d.]+)", s)
    if m:
        d["nlps"] = int(m.group(2)); d["mrows"] = float(m.group(3)); d["itlp"] = float(m.group(5)); d["lpsec"] = float(m.group(6))
    m = re.search(r"cut nnz (\S+) \(rows mean ([\d.]+)\)", s)
    if m:
        d["cutnnz"] = m.group(1)
    m = re.search(r"zh trial: (\d+) rounds, (\d+) cuts \(avg ([\d.]+) nonzeros, rows avg ([\d.]+), zh rows (\d+)/(\d+)\).*g_z (\S+) g_d (\S+) gap (\S+) .*g_z/gap (\S+) g_z/g_d (\S+), fractional (\d+) -> (\d+), (\d+) LP iters \(root LP (\d+)\), sep ([\d.]+)s, root LP ([\d.]+)s; (used|not used) \(([\d.]+)s\)", s)
    if m:
        g = m.groups()
        d["zh"] = dict(rounds=int(g[0]), cuts=int(g[1]), cnnz=float(g[2]), rnnz=float(g[3]), zrows=f"{g[4]}/{g[5]}", gz=float(g[6]), gd=float(g[7]), gap=float(g[8]), gzgap=float(g[9]), gzgd=float(g[10]), frac=f"{g[11]}->{g[12]}", iters=int(g[13]), rootit=int(g[14]), sep=float(g[15]), used=g[17] == "used", secs=float(g[18]))
    return d
def t_of(r):
    res = r["result"]
    return res.get("time") if res.get("status") == "optimal" else None
if __name__ == "__main__":
    by = {}
    for r in rows:
        by.setdefault(r["problem"], []).append(r)
    for p, rs in by.items():
        print(f"## {p}")
        for r in sorted(rs, key=lambda r: (r["cfg"], r["rep"])):
            res = r["result"]; d = logi(r)
            t = t_of(r)
            ts = f"{t:7.1f}" if t is not None else f"TL{r.get('tl', 0):5.0f}"
            z = d.get("zh")
            zs = f" zh: gz/gap {z['gzgap']:.3f} gz/gd {z['gzgd']:.3f} gz {z['gz']:.2e} cnnz {z['cnnz']:.0f}/{z['rnnz']:.0f} zrows {z['zrows']} it {z['iters']}/{z['rootit']} {z['secs']:.2f}s {'USED' if z['used'] else 'no'}" if z else ""
            print(f"  {r['cfg']:6s} s{r['rep']} {ts} rb {res.get('root_bound')!s:>14.14} bd {res.get('bound')!s:>12.12} ob {res.get('objective')!s:>12.12} nodes {res.get('nodes')!s:>7} rc_t {d.get('rc_t', '-')!s:>6} rcuts {d.get('rc_ncuts', '-')!s:>4} mrows {d.get('mrows', '-')!s:>7} it/lp {d.get('itlp', '-')!s:>6} lpsec {d.get('lpsec', '-')!s:>7} cutnnz {d.get('cutnnz', '-')}{zs}")
