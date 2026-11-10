"""根の指標の表と、暫定解と下界の矛盾の検査。使い方: python summ.py OUT [設定,...]
根 (外側の最初の根のカットのループ、ログの最初の `cut loop done` まで): ラウンド数・足した本数・LP に残る本数・その平均非零・終わりの時刻 (秒)。
矛盾: 最小化に直して 下界 > 暫定解 + 1e-6 (1 + |暫定解|)、または解けた回どうしで最適値が 1e-6 (相対) を超えて違う。"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
OUT = sys.argv[1]
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
rows = [json.loads(l) for l in open(D / f"{OUT}.jsonl") if l.strip()]
rx = re.compile(r"cut round (\d+): (\d+) cuts \(of (\d+).*LP rows (\d+) \(cut rows mean nnz (\d+)\), obj (\S+) \((\S+)s\)")
rd = re.compile(r"cut loop done: (\d+) cuts added, (\d+) in LP")

def sense(p):
    try:
        txt = open(WT / ".miplib_cache" / f"{p}.mps", errors="ignore").read(20000)
    except OSError:
        return 1
    m = re.search(r"OBJSENSE\s*\n?\s*(MAX|MAXIMIZE)", txt)
    return -1 if m else 1

def root(p, c, s):
    f = D / "logs" / f"{OUT}_{p}_{c}_s{s}.txt"
    if not f.exists():
        return None
    nr = add = 0; t = 0.0; inlp = nnz = 0
    last_nnz = 0
    for l in f.read_text().splitlines():
        m = rx.search(l)
        if m:
            nr += 1; add += int(m.group(2)); t = float(m.group(7)); last_nnz = int(m.group(5))
            continue
        m = rd.search(l)
        if m:
            inlp = int(m.group(2)); nnz = last_nnz
            break
    return nr, add, inlp, nnz, t

bad = []
opt = defaultdict(dict)
tab = defaultdict(dict)
for r in rows:
    p, c, s, res = r["problem"], r["cfg"], r["rep"], r["result"]
    if cfgs and c not in cfgs:
        continue
    sg = sense(p)
    ob, bd = res.get("objective"), res.get("bound")
    if ob is not None and bd is not None and sg * bd > sg * ob + 1e-6 * (1 + abs(ob)):
        bad.append((p, c, s, "bound>obj", ob, bd))
    if res.get("status") == "optimal" and ob is not None:
        opt[p][(c, s)] = ob
    tab[p][(c, s)] = (res, root(p, c, s))
for p, d in opt.items():
    v = list(d.values())
    if max(v) - min(v) > 1e-6 * (1 + abs(min(v))):
        bad.append((p, "optimal values differ", d))
allc = cfgs or sorted({c for d in tab.values() for (c, _) in d})
print("| 問題 | 設定 | 種 | 状態 | 時間 | 根の下界 | 根のラウンド | 足した本数 | LP に残る本数 | 平均非零 | 根の終わり (秒) |")
print("|---|---|---|---|---|---|---|---|---|---|---|")
for p in tab:
    for c in allc:
        for s in sorted({s for (cc, s) in tab[p] if cc == c}):
            res, rt = tab[p][(c, s)]
            rt = rt or (0, 0, 0, 0, 0.0)
            tm = res.get("time")
            print(f"| {p} | {c} | {s} | {res.get('status')} | {tm:.1f} | {(res.get('root_bound') if res.get('root_bound') is not None else float('nan')):.6g} | {rt[0]} | {rt[1]} | {rt[2]} | {rt[3]} | {rt[4]:.2f} |" if tm is not None else f"| {p} | {c} | {s} | {res.get('status')} | - | - | | | | | |")
print()
print("矛盾:", "なし" if not bad else "")
for b in bad:
    print("  ", b)
