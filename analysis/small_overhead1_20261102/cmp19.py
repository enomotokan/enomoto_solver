"""再スタートのある 19 問で、既定 (a58_base 1 回 + r19_base 2 回) と ENOMOTO_MIP_SMALL1=Wr (r19_s1Wr 2 回) を比べる。
ほかに s1wr (a58_s1wr 1 回) も並べる。時間は解けなければ 60 秒。"""
import json, math, statistics as st
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
G = {"a58_base": "base", "r19_base": "base", "r19_s1Wr": "Wr", "a58_s1wr": "wr"}
R = defaultdict(list)
P = set()
for l in (D / "results.jsonl").read_text().splitlines():
    x = json.loads(l)
    if x["cfg"] == "r19_s1Wr": P.add(x["problem"])
    if x["cfg"] in G:
        r = x["result"]; R[(G[x["cfg"]], x["problem"])].append(r["time"] if r.get("status") == "optimal" and r["time"] <= 60 else 60.0)
sg = lambda v: math.exp(sum(math.log(a + 1) for a in v) / len(v)) - 1
print(f"{'問題':22s} {'既定 (3 回)':>24s} {'Wr (2 回)':>18s} {'wr (1 回)':>9s}")
for p in sorted(P):
    b, w, w1 = R[("base", p)], R[("Wr", p)], R[("wr", p)]
    print(f"{p:22s} {st.median(b):7.2f} [{', '.join(f'{v:.1f}' for v in b)}] {st.median(w):7.2f} [{', '.join(f'{v:.1f}' for v in w)}] {w1[0] if w1 else float('nan'):8.2f}")
mb = [st.mean(R[("base", p)]) for p in sorted(P)]; mw = [st.mean(R[("Wr", p)]) for p in sorted(P)]
print(f"19 問の SGM (各問の平均時間): 既定 {sg(mb):.3f}  Wr {sg(mw):.3f}  比 {sg(mw) / sg(mb):.3f}")
