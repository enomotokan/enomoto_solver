"""e.jsonl (ENOMOTO) と h.jsonl (HiGHS) から記録の表を作る。
使い方: python tab.py e [設定...]  /  python tab.py h"""
import json, re, sys
from collections import defaultdict
P = ["supportcase33", "ns1830653", "rococoC10-001000", "traininstance6", "tr12-30"]
if sys.argv[1] == "h":
    C = ["def", "cut", "nosepnodes", "age100", "rel0", "noheur", "norestart"]
    R = {}
    for l in open("h.jsonl"):
        r = json.loads(l); R[(r["problem"], r["cfg"])] = r
    print("| 問題 | " + " | ".join(C) + " |"); print("|---" * (len(C) + 1) + "|")
    for p in P:
        cells = []
        for c in C:
            r = R.get((p, c))
            cells.append("欠" if not r or r.get("nodes") is None else f"{r['nodes']} / {r['it_per_node']}")
        print(f"| {p} | " + " | ".join(cells) + " |")
    sys.exit()
cfgs = sys.argv[2:]
rows = [json.loads(l) for l in open("e.jsonl")]
print("| 問題 | 設定 | 種 | ノード | 暫定解 | 下界 | 反復/LP | 潜りの子 | 兄弟 | 待ち行列 | 解き直し | ms/LP | カットの行 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in P:
    for c in cfgs:
        for r in rows:
            if r["problem"] != p or r["cfg"] != c:
                continue
            agg = defaultdict(lambda: [0.0] * 4)
            for l in r["nodelp"]:
                m = re.match(r"MIP: nodelp tid (\d+) (\w+): n (\d+) iters ([\d.]+)/LP ms ([\d.]+)/LP rows ([\d.]+) cutrows ([\d.]+)", l)
                if not m:
                    continue
                n = int(m.group(3))
                for k in (m.group(2), "all"):
                    a = agg[k]; a[0] += n; a[1] += n * float(m.group(4)); a[2] += n * float(m.group(5)); a[3] += n * float(m.group(7))
            f = lambda k: f"{agg[k][1]/agg[k][0]:.0f}" if agg[k][0] else ""
            res = r["result"]; ob = res.get("objective"); bd = res.get("bound")
            print(f"| {p} | {c} | {r['rep']} | {res.get('nodes')} | {'' if ob is None else round(ob, 1)} | {'' if bd is None else round(bd, 1)} | {f('all')} | {f('dive')} | {f('sibling')} | {f('queue')} | {f('resolve')} | "
                  f"{agg['all'][2]/max(agg['all'][0],1):.1f} | {agg['all'][3]/max(agg['all'][0],1):.0f} |")
