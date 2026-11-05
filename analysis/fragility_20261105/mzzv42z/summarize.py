"""ログ (log_*.txt) と結果 (.stdout) から表を作る。使い方: python summarize.py [ラベルの接頭辞]"""
import glob, json, re, sys, os
pre = sys.argv[1] if len(sys.argv) > 1 else ""
d = os.path.dirname(os.path.abspath(__file__))
rows = []
for f in sorted(glob.glob(f"{d}/log_{pre}*.txt")):
    lab = os.path.basename(f)[4:-4]
    try:
        r = json.load(open(f + ".stdout"))
    except Exception:
        r = {}
    L = open(f, errors="replace").read().splitlines()
    rs = None; restart_t = None; tree_t = None; incs = []; root_lp = None
    for l in L:
        m = re.match(r"MIP: restart 1 .*\((\d+\.\d+)s\)", l)
        if m: restart_t = float(m.group(1)); continue
        m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \((\d+\.\d+)s\) \[(.*)\]", l)
        if m:
            t = float(m.group(3)) + (restart_t or 0.0)
            incs.append((float(m.group(1)), int(m.group(2)), t, m.group(4)))
            continue
        m = re.match(r"MIP: root LP after the restart: \w+, (\d+) iters \((\d+\.\d+)s\)", l)
        if m: root_lp = (int(m.group(1)), float(m.group(2)))
        m = re.match(r"MIP: tree search with \d+ threads \((\d+\.\d+)s\)", l)
        if m: tree_t = float(m.group(1)) + (restart_t or 0.0)
    nd = re.findall(r"node diag: nodes (\d+) node LPs (\d+) .* iters (\d+) \((\S+)/LP\) LP secs (\S+)", "\n".join(L))
    fr = re.findall(r"tree first LPs restored (\d+) iters (\d+) \((\S+)/LP\)", "\n".join(L))
    # 木の始め (再スタートがあれば再スタート後の木の始め) までに見つかった最良の暫定解
    pre_inc = sorted([i for i in incs if tree_t is None or i[2] <= tree_t + 1e-9])
    n_alns = sum(1 for l in L if "ALNS arm" in l)
    rob = [l for l in L if l.startswith("MIP: robust stall:")]
    n_rob = (len(rob), sum(1 for l in rob if "accepted true" in l))
    best = min(incs) if incs else None
    opt = [i for i in incs if abs(i[0] + 20540) < 0.5]
    rows.append((lab, r.get("time"), r.get("nodes"), r.get("status"), r.get("objective"), restart_t,
                 pre_inc[0][0] if pre_inc else None, root_lp, tree_t,
                 (f"{opt[0][2]:.0f}s [{opt[0][3]}]" if opt else "-"),
                 nd[-1] if nd else None, fr[-1][2] if fr else None, n_alns, n_rob))
print("| ラベル | 時間 | ノード | 状態 | 再スタート時刻 | 木の始めの暫定解 | 再スタート後の根の LP (反復/秒) | 木の始め | -20540 を見つけた時刻 [出どころ] | 木の LP 回数 | 1 LP の反復 | 復元後の最初の LP の反復 | ALNS 回数 | robust 回数 (改善) |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in rows:
    lab, t, n, st, obj, rt, pi, rl, tt, op, nd, fr, na, nr = r
    print(f"| {lab} | {t and round(t)} | {n} | {st} | {rt} | {pi} | {rl and f'{rl[0]}/{rl[1]}'} | {tt and round(tt)} | {op} | {nd and nd[1]} | {nd and nd[3]} | {fr} | {na} | {nr[0]} ({nr[1]}) |")
