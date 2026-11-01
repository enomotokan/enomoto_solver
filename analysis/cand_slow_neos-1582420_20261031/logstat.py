"""ログから外側の求解の様子を集計する (再スタートの区間ごと)。
区間ごとに: 根のカットのループの終わりの目的値、根のヒューリスティクスの終わりの時刻と暫定解、裏のカットのラウンドと目的値、
再スタートの固定数、区間の中の暫定解の更新 (値@時刻[出どころ])。最後に木のノード・全体の時間。
使い方: python logstat.py <ログ>... (-v で根のサブ MIP の一覧も)"""
import re, sys
V = "-v" in sys.argv
for f in [a for a in sys.argv[1:] if a != "-v"]:
    s = open(f).read()
    m = re.findall(r'"time": ([\d.]+)', s)
    rb = re.findall(r'"root_bound": ([\d.e+-]+)', s)
    nd = re.findall(r'"nodes": (\d+)', s)
    segs = re.split(r"^MIP: restart \d+ \(", s, flags=re.M)
    out = []
    for i, g in enumerate(segs):
        cl = re.findall(r"cut loop done: .*obj [\d.e+-]+ -> ([\d.e+-]+)", g)
        rh = re.findall(r"after root heuristics: incumbent (\S+) \(([\d.]+)s\)", g)
        bg = re.findall(r"background cuts: (\d+) rounds, \d+ cut rows .*-> ([\d.e+-]+) \(waited ([\d.]+)s", g)
        inc = re.findall(r"new incumbent ([\d.e+-]+) at node \d+ \(([\d.]+)s\) \[([^\]]*)\]", g)
        fx = re.findall(r"^([\d]+) of \d+ integer columns newly fixed", g, re.M)
        sub = [float(t) for t in re.findall(r"sub-MIP(?: \([^)]*\))?: (?:\d+ free columns of \d+, )?status \w+, nodes \d+, objective [^,]*(?:\([^)]*\))?, ([\d.]+)s", g.split("after root heuristics")[0])]
        ps = "PS" if "reached the prune limit" in g else ""
        tr = re.findall(r"finish done \(([\d.]+)s\)", g)
        incs = " ".join(f"{float(v):g}@{float(t):.1f}" + ("" if src == "tree" else f"[{src[:12]}]") for v, t, src in inc if not (float(t) == 0.0 and src == "tree"))
        out.append(f"  区間{i}{'(固定' + fx[0] + ')' if fx else ''}: cut {float(cl[0]) if cl else 0:.3f} heur {rh[0][1] if rh else '-'}s sub {len(sub)}本{sum(sub):.1f}s "
                   f"bg {bg[0][0] + 'r ' + format(float(bg[0][1]), '.3f') + ' w' + bg[0][2] if bg else '-'} {ps} 解 {incs}" + (f" 終{tr[0]}" if tr else ""))
    lp = re.search(r"^MIP: root LP [\d.e+-]+ \((\d+) iters", s, re.M)
    cl0 = re.search(r"cut loop done: .*-> ([\d.e+-]+)", s)
    # neos-1582420 の根の道: 最初の根のカットのループが 89.251 で終われば A、89.261 なら B (根の LP の頂点で決まる)
    path = {"8.9251127551e1": "A", "8.9261487563e1": "B"}.get(cl0.group(1) if cl0 else "", "?")
    print(f"{f.split('/')[-1]:40s} {float(m[-1]) if m else -1:5.1f}s nodes {nd[-1] if nd else '?'} rootb {float(rb[-1]) if rb else 0:.3f} 道 {path} 根LP {lp.group(1) if lp else '?'}反復")
    print("\n".join(out))
