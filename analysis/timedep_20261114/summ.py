"""A/A のログから、時間で決まりうる分かれ道を 1 回ごとに拾って並べる: python summ.py [問題,...] [--md]
各回の行: 結果 (状態・秒・ノード・根の下界) と、分かれ道の印 (下の PATS)。同じ (問題, スレッド数) の回の間で値が違う印に * を付ける。"""
import re, sys
from pathlib import Path
from collections import defaultdict
D = Path(__file__).resolve().parent / "logs"
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and not sys.argv[1].startswith("--") else None
MD = "--md" in sys.argv


def feats(txt):
    L = [l for l in txt.splitlines() if "slope-intercept" not in l]
    f = {}
    m = [l for l in L if l.startswith("RESULT")]
    if m:
        p = m[-1].split()
        f["res"] = f"{p[1]} {float(p[6]):.1f}s n{p[4]} rb{float(p[5]):.6g}" if p[5] != "None" else f"{p[1]} {float(p[6]):.1f}s n{p[4]}"
    else:
        f["res"] = "running"
    # 根の LP の同時実行 (E1): どちらが先か
    f["rootlp"] = ",".join("ipm" if "interior-point" in l else "spx" for l in L if l.startswith("Root LP: the "))
    # カット選びの比べ (CUTSEL_PROBE)
    f["cutsel"] = ",".join(re.search(r"-> (\w+)", l).group(1) for l in L if "cut selection probe (" in l)
    f["capstop"] = str(sum(1 for l in L if "root cut time cap check" in l and "stop" in l.split("cap check")[1]))
    f["probe_m"] = ",".join(re.search(r"compared at (\d+ rounds \([^)]*\))", l).group(1) for l in L if "cut selection probe: compared" in l)
    # 根のカット: 各段の (ラウンド数, 最後の値)
    rc, cur = [], 0
    for l in L:
        if re.match(r"MIP: cut round \d+: \d+ cuts", l):
            cur += 1
        if l.startswith("Root cuts:"):
            v = re.search(r"-> ([-\d.e+]+)", l).group(1)
            rc.append(f"{cur}r:{float(v):.6g}"); cur = 0
    f["rootcuts"] = " ".join(rc)
    # 再スタート (早め・普通・木の中)
    f["restart"] = ",".join(re.sub(r"\s*\(.*", "", l)[:40] for l in L if re.match(r"(Restart \d+:|MIP: early restart|MIP: tree restart|TREER)", l))[:120]
    # 裏のカット
    f["bgcut"] = " ".join(re.sub(r".*?\(([^)]*)\), (\d+) rounds.*", r"\1/\2r", l).replace("safety cap ", "")
                          for l in L if "background cuts: determ: waited" in l)
    f["erwait"] = " ".join(re.sub(r".*ERWAIT: ", "", l)[:60] for l in L if "ERWAIT" in l)
    # 根の暫定解の出どころ (最初の 3 つ)
    inc = [re.search(r"new incumbent ([-\d.e+]+) at node (\d+) \(([\d.]+)s\) \[([^\]]+)\]", l) for l in L if "new incumbent" in l]
    inc = [x for x in inc if x]
    f["inc1"] = " ".join(f"{float(x.group(1)):.6g}@{x.group(4)}" for x in inc[:3])
    f["ninc"] = str(len(inc))
    f["fjonly"] = ",".join(re.search(r"FJ thread (\w+)", l).group(1) for l in L if "LMBOOST:" in l and "FJ thread" in l)
    f["lmboost"] = ",".join("boost" if "boosting" in l else "no" for l in L if "LMBOOST:" in l)
    # 主の sub-MIP の状態 (TimeLimit の数 / 全部)
    sm = [re.search(r"status (\w+)", l).group(1) for l in L if re.search(r"sub-MIP.*status", l)]
    f["submipTL"] = f"{sm.count('TimeLimit')}/{len(sm)}"
    f["heurfound"] = ",".join(re.search(r"root heuristic (.*?): found", l).group(1) for l in L if re.search(r"root heuristic .*: found", l))
    f["ipmround"] = ",".join(re.sub(r".*interior rounding: ", "", l)[:30] for l in L if "interior rounding: " in l and "points" in l and "rounded" not in l)
    f["gap1"] = " ".join(re.sub(r"MIP: GAP1 u: ", "", l)[:50] for l in L if "GAP1 u: LP probing" in l and "skipped" not in l)
    f["lpscale"] = " ".join(l[:60] for l in L if "LPSCALE" in l)[:120]
    f["guard"] = " ".join(re.search(r"guard (stop|pass)\S*", l).group(0).split()[1] for l in L if re.search(r"guard (stop|pass)", l))[:120]
    return f


rows = defaultdict(list)
for p in sorted(D.glob("*.log")):
    m = re.match(r"(.+)_t(\d+)_([a-z]+)(\d+)\.log", p.name)
    if not m: continue
    if probs and m.group(1) not in probs: continue
    rows[(m.group(1), int(m.group(2)))].append((m.group(3) + m.group(4), feats(p.read_text(errors="replace"))))
for key in sorted(rows):
    rs = rows[key]
    ks = list(rs[0][1].keys())
    diff = {k for k in ks if len({r[1].get(k) for r in rs}) > 1}
    print(f"\n## {key[0]} {key[1]} スレッド")
    for tag, f in rs:
        s = " | ".join(f"{('*' if k in diff else '')}{k}={f[k]}" for k in ks if f[k])
        print(f"{tag}: {s}")
