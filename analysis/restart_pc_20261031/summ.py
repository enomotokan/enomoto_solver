"""集計: (1) 問題 × 設定の時間 (秒、T は時間切れ。res.jsonl)、(2) `-v` でログごとに、再スタートの回数、根のヒューリスティクスの終わりの時刻
(再スタートごと。再スタートで時計が戻る)、外側から出したサブ MIP の時間の和、再スタート時の pseudocost の観測 (両向きの列 / 観測の総数)、
最終の暫定解を最初に出した区間 (r0 は最初の求解、r1 は 1 回目の再スタート後) と時刻・出どころ。
使い方: python summ.py [問題,...] [設定,...] [-v]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
args = [a for a in sys.argv[1:] if a != "-v"]
probs = args[0].split(",") if len(args) > 0 and args[0] else None
cfgs = args[1].split(",") if len(args) > 1 else ["def", "hs_pi0_bgw", "hs_pi0_bgw_r", "hs_pi0_bgw_pc", "hs_pir_bgw_pc"]
res = defaultdict(dict)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if r["rep"] >= 90:
        continue
    try:
        o = json.loads(r["out"])
        t = o["time"] if o.get("status") == "optimal" else None
    except Exception:
        t = None
    res[(r["prob"], r["cfg"])][r["rep"]] = t
order = list(dict.fromkeys(p for p, _ in res))
probs = probs or order
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    cells = []
    for c in cfgs:
        v = res.get((p, c), {})
        cells.append("・".join("T" if v[k] is None else f"{v[k]:.1f}" for k in sorted(v)))
    print(f"| {p} | " + " | ".join(cells) + " |")
if "-v" in sys.argv:
    print()
    for p in probs:
        for c in cfgs:
            for f in sorted((D / "logs").glob(f"{p}_{c}_[0-9].log")):
                s = f.read_text()
                segs = re.split(r"^MIP: restart \d+ \(", s, flags=re.M)
                rh = [re.findall(r"^MIP: after root heuristics: .*\(([\d.]+)s\)", g, re.M) for g in segs]
                rh = "+".join(f"{float(x[0]):.1f}" if x else "-" for x in rh)
                sub = sum(float(t) for t in re.findall(r"sub-MIP(?: \([^)]*\))?: (?:\d+ free columns of \d+, )?status \w+, nodes \d+, objective [^,]*(?:\([^)]*\))?, ([\d.]+)s", s))
                pc = ["%s/%d" % (b, int(u) + int(d)) for b, u, d in re.findall(r"pseudocost observations: columns up \d+, down \d+, both (\d+) .*?observations up (\d+), down (\d+)", s)]
                inc = [(k, m) for k, g in enumerate(segs) for m in re.findall(r"new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*?)\]", g)]
                src = ""
                if inc:
                    z = float(inc[-1][1][0])
                    k, m = next((k, m) for k, m in inc if abs(float(m[0]) - z) <= 1e-6 * max(1, abs(z)))
                    src = f"r{k} {m[2]}s n{m[1]} [{m[3]}]"
                t = re.findall(r'"time": ([\d.]+)', s)
                st = re.findall(r'"status": "(\w+)"', s)
                print(f"{f.stem:34s} {st[-1][:4] if st else '?'} {float(t[-1]) if t else -1:5.1f}  再 {len(segs) - 1} 根ヒュ {rh:20s} subMIP {sub:5.1f}s  pc {','.join(pc) or '-':12s} 最良 {src}")
