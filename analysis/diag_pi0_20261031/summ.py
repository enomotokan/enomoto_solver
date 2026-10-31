"""ログ (<問題>_<設定>_<回>.log) の集計: 時間・ノード・サブ MIP ごとの時間とノード・暫定解の出どころ。
使い方: python summ.py [問題,...] [設定,...] [-v]"""
import json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent

def parse(path):
    t = path.read_text()
    last = [l for l in t.splitlines() if l.startswith("{")]
    res = json.loads(last[-1]) if last else {}
    sub = defaultdict(lambda: [0, 0.0, 0])  # 名前 -> [回数, 秒, ノード]
    label = None
    for l in t.splitlines():
        m = re.match(r"MIP:   parallel sub-MIP \((.+?)\): status (\w+), nodes (\d+), .*?, ([\d.]+)s", l)
        if m:
            s = sub[m.group(1)]; s[0] += 1; s[1] += float(m.group(4)); s[2] += int(m.group(3)); continue
        m = re.match(r"MIP:   sub-MIP: .*status (\w+), nodes (\d+), .*?, ([\d.]+)s", l)
        if m:
            label = "serial"
            pend = (int(m.group(2)), float(m.group(3))); continue
        m = re.match(r"MIP: root heuristic (.+?): .*\(([\d.]+)s\)", l)
        if m and label == "serial":
            s = sub[m.group(1)]; s[0] += 1; s[1] += pend[1]; s[2] += pend[0]; label = None; continue
        m = re.match(r"MIP: (?:tree )?heuristic (.+?): ", l)
        if m and label == "serial":
            s = sub[m.group(1)]; s[0] += 1; s[1] += pend[1]; s[2] += pend[0]; label = None; continue
    if label == "serial":
        s = sub["?"]; s[0] += 1; s[1] += pend[1]; s[2] += pend[0]
    inc = re.findall(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.+?)\]", t)
    restarts = len(re.findall(r"MIP: restart \d+ \(", t))
    m = re.search(r"branching: (\d+) selections.*sb LPs (\d+) iters (\d+)", t)
    sb = (int(m.group(2)), int(m.group(3))) if m else (0, 0)
    stops = defaultdict(int)
    for why in re.findall(r"sub-MIP stop \((.+?)\)", t):
        stops[why] += 1
    return res, dict(sub), inc, restarts, sb, dict(stops)

def main():
    args = [a for a in sys.argv[1:] if a != "-v"]
    verbose = "-v" in sys.argv
    probs = args[0].split(",") if args else None
    cfgs = args[1].split(",") if len(args) > 1 else None
    rows = defaultdict(list)
    for f in sorted(D.glob("*.log")):
        m = re.match(r"(.+)_(\w+?)_(\d+)\.log$", f.name)
        # 設定名に _ を含むので、既知の問題名で分ける
        name = f.name[:-4]
        p, rest = None, None
        for q in (probs or []):
            if name.startswith(q + "_"):
                p, rest = q, name[len(q) + 1:]
        if p is None:
            if probs: continue
            p, rest = re.match(r"(.+?)_(.+)$", name).groups()
        c, rep = rest.rsplit("_", 1)
        if cfgs and c not in cfgs: continue
        rows[(p, c)].append((int(rep), parse(f)))
    for (p, c), lst in sorted(rows.items()):
        ts = []
        for rep, (res, sub, inc, rs, sb, stops) in sorted(lst):
            st = res.get("status", "?")
            tt = res.get("time", float("nan"))
            ts.append(tt if st == "optimal" else 60.0)
            if verbose:
                subs = " ".join(f"{k}:{v[0]}x/{v[1]:.1f}s/{v[2]}n" for k, v in sorted(sub.items(), key=lambda kv: -kv[1][1]))
                incs = " ".join(f"{src}@{float(tm):.1f}" for _, _, tm, src in inc)
                print(f"  {p} {c} {rep}: {st} {tt:.1f}s nodes {res.get('nodes')} restarts {rs} sbLP {sb[0]}/{sb[1]}it stops {stops} | sub {subs} | inc {incs}")
        print(f"{p:16s} {c:14s} " + " ".join(f"{x:5.1f}" for x in ts))
main()
