"""問題ごとに、回ごとの道しるべ (根のカットのラウンド数・終わりの値・再スタート数・最良解を最初に得た時刻と出どころ・ノード数・秒) を並べる:
python paths.py スレッド 問題..."""
import sys, glob, json
from pathlib import Path
from digest import digest
D = Path(__file__).resolve().parent
th = sys.argv[1]
res = {}
for l in open(D / f"res_t{th}.jsonl"):
    r = json.loads(l); res[(r["problem"], f"p{r['perm']}s{r['seed']}")] = r["result"]
for p in sys.argv[2:]:
    print(f"### {p}")
    for f in sorted(glob.glob(str(D / f"logs/{p}_t{th}_p*s*.log.gz"))):
        v = f.split(f"_t{th}_")[1][:-7]; d = digest(f); r = res.get((p, v), {})
        nr = {}
        for rr, _ in d["rounds"]: nr[rr] = nr.get(rr, 0) + 1
        best = min((x[3] for x in d["inc"]), default=None) if r.get("objective") is None else r["objective"]
        first = next((x for x in d["inc"] if abs(x[3] - best) <= 1e-6 * max(1, abs(best))), None) if best is not None else None
        st = "T" if r.get("status") != "optimal" else ""
        print(f"  {v}: {st}{(r.get('time') or 0):.1f}s nodes {r.get('nodes')} rounds {list(nr.values())} cutend {['%.6g' % x for x in d['cutend']]} restarts {d['restarts']} "
              f"best {best if best is None else '%.6g' % best} first {first and f'R{first[0]}:{first[1]:.1f}s[{first[2]}]@{first[4]}'} rootbound {r.get('root_bound') and '%.6g' % r['root_bound']}")
