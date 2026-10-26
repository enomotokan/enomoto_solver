"""(b) の要約: python sum_b.py -> 問題・形・回ごとに暫定解・下界・木に入った時刻・根のサブ MIP の時間の和 (回数)・解を出したサブ MIP。"""
import glob, json, re
for p in "physiciansched6-2 mzzv42z neos-960392 30n20b8 bppc4-08 timtab1".split():
    for n in ("def", "h1", "hnt"):
        for f in sorted(glob.glob(f"logs/{p}_{n}_*.log")):
            sub = 0.0; k = 0; tree = None; found = []
            for line in open(f, errors="replace"):
                m = re.search(r"sub-MIP(?: \([^)]*\))?: .*?([\d.]+)s$", line)
                if m and ("MIP:   sub-MIP:" in line or "parallel sub-MIP (" in line):
                    sub += float(m.group(1)); k += 1
                m = re.search(r"tree search with \d+ threads \(([\d.]+)s\)", line)
                if m and tree is None:
                    tree = float(m.group(1))
                m = re.search(r"new incumbent (\S+) at node 0 \(([\d.]+)s\) \[(.*)\]", line)
                if m and m.group(3) not in ("Local-MIP", "feasibility jump"):
                    found.append(f"{float(m.group(1)):.6g}@{float(m.group(2)):.0f}s {m.group(3)}")
                if line.startswith("{"):
                    d = json.loads(line)
            print(f"{p} | {n} | {f.split('_')[-1][:-4]} | {d['objective'] if d['objective'] is None else format(d['objective'], '.6g')} | {d['best_bound']:.6g} | {tree} | {sub:.1f} ({k}) | {'; '.join(found[-3:])}")
