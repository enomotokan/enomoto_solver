"""数えの比べ (今の判定 ri と ネットワークの判定 rin): 外した組・認めた組・畳んだ大きさ・時間。python summ_count.py ri.jsonl rin.jsonl [...]"""
import json, re, sys
def load(p):
    out = {}
    for l in open(p):
        d = json.loads(l); g = {}
        for x in d["fold"]:
            m = re.search(r"(\d+) accepted, (\d+) rejected", x)
            if m: g["acc"], g["rej"] = int(m[1]), int(m[2])
            m = re.search(r"by net (\d+) work (\d+) over (\d+)", x)
            if m: g["net"], g["work"], g["over"] = int(m[1]), int(m[2]), int(m[3])
            m = re.search(r"MIP_FOLD: n (\d+) -> (\d+), m (\d+) -> (\d+)", x)
            if m: g["fold"] = f"n {m[1]}->{m[2]} m {m[3]}->{m[4]}"
            t = re.findall(r" in ([\d.]+)s", x)
            if t: g["t"] = float(t[-1])
            if "no fold" in x and "fold" not in g: g["fold"] = "-"
        out[d["name"]] = g
    return out
A = [load(p) for p in sys.argv[1:]]
names = sorted(set(A[1]))
print("| 問題 | " + " | ".join(f"{p.split('/')[-1]}: 認めた/外した・畳んだ大きさ・秒" for p in sys.argv[1:]) + " |")
print("|---|" + "---|" * len(A))
tot = [0.0] * len(A)
for n in names:
    cells = []
    for i, a in enumerate(A):
        g = a.get(n, {})
        tot[i] += g.get("t", 0)
        cells.append(f"{g.get('acc','')}/{g.get('rej','')} {g.get('fold','')} {g.get('t','')}" + (f" (ネットで認めた {g['net']}, 仕事 {g['work']}, 上限超え {g['over']})" if "work" in g else ""))
    print(f"| {n} | " + " | ".join(cells) + " |")
print("合計の秒: " + ", ".join(f"{t:.1f}" for t in tot))
