"""scan2.jsonl と scan.jsonl (広げた走査の分は ORBDIAG の行つき) から、ORBPP の列の下限 (既定 10) で外れる「列 3〜9 のパッキング・分割の 0-1 のオービトープ」を持つ問題を拾う。
条件は src/mip/mod.rs の ORBPP と同じ (local でない、0-1、パッキングの行が 3 行以上かつ過半)。python cands.py → cands.json と表"""
import json, re
from pathlib import Path
D = Path(__file__).resolve().parent
RX = re.compile(r"ORBDIAG struct orbitope (\d+) (\d+)x(\d+) binary (\w+) local (\w+) packing_rows (\d+) partition_rows (\d+) extra_in_rows (\d+) cols_cover (\d+) of free (\d+) \(binary (\d+)\) abs_cost (\S+)")
out = []
seen = set()
for l in (D / "scan2.jsonl").read_text().splitlines() + (D / "scan.jsonl").read_text().splitlines():
    r = json.loads(l)
    if r["problem"] in seen: continue
    orbs = []
    for x in r.get("lines", []):
        m = RX.search(x)
        if not m: continue
        k, R, C, b, loc, pk, part, extra, cov, free, nbin, cost = m.groups()
        orbs.append(dict(k=int(k), rows=int(R), cols=int(C), binary=b == "true", local=loc == "true", pk=int(pk), part=int(part), extra=int(extra), cover=int(cov), free=int(free), nbin=int(nbin), cost=float(cost)))
    if not orbs: continue
    seen.add(r["problem"])
    elig = [o for o in orbs if o["binary"] and not o["local"] and o["pk"] >= 3 and 2 * o["pk"] >= o["rows"]]
    small = [o for o in elig if 3 <= o["cols"] < 10]
    big = [o for o in elig if o["cols"] >= 10]
    if small:
        out.append(dict(problem=r["problem"], status=r["status"], nnz=r["nnz"], tags=r["tags"], orbs=orbs, small=[o["k"] for o in small], big=[o["k"] for o in big]))
        print(r["problem"], r["status"], r["nnz"], "small", [f'{o["pk"]}/{o["rows"]}x{o["cols"]} part{o["part"]} extra{o["extra"]}' for o in small][:8], "big", len(big), "all", len(orbs), "free", small[0]["free"])
(D / "cands.json").write_text(json.dumps(out, indent=0))
print(len(out))
