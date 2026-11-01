"""results.jsonl とログから、設定ごとの解けた回数・時間と、回ごとの経路 (根の RENS・最適解の出どころ) を出す。
使い方: python summ.py [問題 (既定 neos-960392)]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
prob = sys.argv[1] if len(sys.argv) > 1 else "neos-960392"
rows = [json.loads(l) for l in open(D / "results.jsonl")]
rows = [r for r in rows if r["prob"] == prob]
by = defaultdict(list)
for r in rows:
    by[(r["cfg"], r["tl"])].append(r)
print(f"{'設定':16s} {'上限':>4s} {'解けた':>6s}  時間 (解けた回)  / 解けなかった回の暫定解")
for (c, tl), rs in by.items():
    ok = [r for r in rs if r.get("status") == "optimal"]
    ng = [r for r in rs if r.get("status") != "optimal"]
    print(f"{c:16s} {tl:4d} {len(ok)}/{len(rs):<4d}  {' '.join(f'{r['time']:.1f}' for r in sorted(ok, key=lambda r: r['time'])):30s} / {' '.join(str(r.get('objective')) for r in ng)}")
print()
pat_cut = re.compile(r"cut loop done: (\d+) cuts added, (\d+) in LP")
pat_rs = re.compile(r"parallel sub-MIP started \(RENS\): (\d+) free")
pat_rr = re.compile(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+), objective (\S+), ([\d.]+)s")
pat_inc = re.compile(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[([^\]]+)\]")
pat_tree = re.compile(r"tree search with (\d+) threads \(([\d.]+)s\)")
pat_ret = re.compile(r"solve returned (\w+) after ([\d.]+)s")
pat_rst = re.compile(r"MIP: restart(?:ing)?[^\n]*")
for r in rows:
    tag = r["cfg"].replace("@", "_th") + (f"_t{r['tl']}" if r["tl"] != 60 else "")
    f = D / f"{prob}_{tag}_{r['rep']}.log"
    if not f.exists():
        continue
    s = f.read_text()
    cut = pat_cut.findall(s)
    rs = pat_rs.findall(s)
    rr = pat_rr.findall(s)
    inc = [m for m in pat_inc.findall(s)]
    best = inc[-1] if inc else None
    tree = pat_tree.findall(s)
    ret = pat_ret.findall(s)
    nrst = len(re.findall(r"MIP: restart(?! check)", s))
    print(f"{tag:16s} {r['rep']} {r.get('status','?'):10s} {r.get('time',0):5.1f}s  cuts {','.join(a+'/'+b for a,b in cut):10s} "
          f"RENS {','.join(rs):12s} {';'.join(f'{a} n{b} {c} {d}s' for a,b,c,d in rr):40s} "
          f"最終解 {best[0] if best else '-'} @{best[2] if best else '-'}s node {best[1] if best else '-'} [{best[3] if best else '-'}] "
          f"木 {tree[0][1] if tree else '-'}s 再スタート {nrst}")
