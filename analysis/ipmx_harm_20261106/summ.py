"""res.jsonl (run.py の結果) を問題 × 設定の表にする。使い方: python summ.py [設定の接尾辞 (例 _v2)]
時間は `time` (m.solve だけ)。60 秒を超えて返った回は (+) を付ける。根の下界と、ログがあれば根の LP の競争の勝ち (s/i) も出す。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
tag = sys.argv[1] if len(sys.argv) > 1 else "_v2"
R = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if not r["cfg"].endswith(tag):
        continue
    R[(r["problem"], r["cfg"][: -len(tag)] if tag else r["cfg"])].append(r)


def race(p, c, rep):
    f = D / "logs" / f"{p}_{c}{tag}_{rep}.log"
    if not f.exists():
        return "?"
    s = f.read_text(errors="replace")
    # 根の LP の競争 (LP の探りの後の、最初の主の根)。勝ちの行を順に s/i で並べる
    w = re.findall(r"MIP root race: (simplex won|ipm_crossover won)", s)
    return "".join("s" if x.startswith("simplex") else "i" for x in w)


probs = list(dict.fromkeys(k[0] for k in R))
cfgs = list(dict.fromkeys(k[1] for k in R))
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    cells = []
    for c in cfgs:
        rs = sorted(R.get((p, c), []), key=lambda r: r["rep"])
        xs = []
        for r in rs:
            res = r["result"]
            t = res.get("time") or 0
            st = res.get("status")
            mark = "" if st == "optimal" else ("TL" if st == "time_limit" else st)
            over = "(+)" if t > 61 else ""
            rb = res.get("root_bound")
            rbs = f" rb {rb:.6g}" if isinstance(rb, (int, float)) else ""
            ob = res.get("objective")
            obs = f" ub {ob:.6g}" if isinstance(ob, (int, float)) and st != "optimal" else ""
            xs.append(f"{t:.1f}{mark}{over} [{race(p, c, r['rep'])}]{rbs}{obs}")
        cells.append("<br>".join(xs))
    print(f"| {p} | " + " | ".join(cells) + " |")
