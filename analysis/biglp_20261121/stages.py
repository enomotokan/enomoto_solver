"""当方のログ (run.py が各行に経過秒を付けたもの) から、根までの段の時刻を拾って並べる。
使い方: python stages.py logs/<問題>_ours4.log ...   段: 前処理の終わり (根の LP を始める)、LP による固定、根の LP、カットの選び方の比べ、
カットのループ (ラウンド数・LP 反復・分離の秒)、根のヒューリスティクス (長いものの上位)、裏のカットの待ち、再スタート、木の始まり (ノード 1 以上)。"""
import re, sys
for fn in sys.argv[1:]:
    L = open(fn, errors="replace").read().splitlines()
    T = lambda l: float(l[1:9])
    ev = []; rounds = []; heur = []; npass = 0
    for l in L:
        if not l.startswith("["): continue
        t, s = T(l), l[11:]
        m = re.search(r"MIP: LP probing: root LP ([0-9.]+)s \((\d+) iters.*total (\d+) iters ([0-9.]+)s", s)
        if m: ev.append((t, f"LP による固定 計 {m.group(4)} 秒 (最初の LP {m.group(1)} 秒・{m.group(2)} 反復、計 {m.group(3)} 反復)"))
        m = re.search(r"MIP: LP probing: the first LP stopped: (\w+) \(([0-9.]+)s", s)
        if m: ev.append((t, f"LP による固定: 最初の LP を {m.group(1)} で止めた ({m.group(2)} 秒)"))
        if "MIP: solving the root LP" in s: npass += 1; ev.append((t, f"根の LP を始める (回 {npass})"))
        m = re.search(r"MIP: root LP (?:after the restart: \w+, )?([-0-9.e]+) \((\d+) iters, ([0-9.]+)s\)", s)
        if m and "after the restart" not in s: ev.append((t, f"根の LP 終わり {m.group(2)} 反復・{m.group(3)} 秒 (値 {float(m.group(1)):.8g})"))
        m = re.search(r"cut selection probe: compared at (\d+) rounds \(([^)]*)\), probe wall ([0-9.]+)s", s)
        if m: ev.append((t, f"カットの選び方の比べ 壁 {m.group(3)} 秒 (比べたラウンド {m.group(1)}、{m.group(2)})"))
        m = re.search(r"cut round (\d+): (\d+) cuts \(of (\d+), sep ([0-9.]+)s, LP (\d+) iters\), LP rows (\d+) \(cut rows mean nnz (\d+)\), obj ([-0-9.e]+)", s)
        if m: rounds.append((t, int(m.group(5)), float(m.group(4)), int(m.group(7))))
        m = re.search(r"MIP: cut loop done: .*obj ([-0-9.e]+) -> ([-0-9.e]+)", s)
        if m:
            r = rounds; rounds = []
            ev.append((t, f"カットのループ終わり: {len(r)} ラウンド、LP {sum(x[1] for x in r)} 反復、分離 {sum(x[2] for x in r):.1f} 秒、カットの平均非零 {max((x[3] for x in r), default=0)} (最大)、値 {float(m.group(1)):.8g} → {float(m.group(2)):.8g}"))
        m = re.search(r"MIP: root heuristic (.+?): (\w+).*\(([0-9.]+)s\)$", s)
        if m and float(m.group(3)) >= 3: heur.append((float(m.group(3)), m.group(1), m.group(2)))
        m = re.search(r"parallel sub-MIP \((.+?)\): status (\w+), nodes (\d+), .*?, ([0-9.]+)s$", s)
        if m and float(m.group(4)) >= 3: heur.append((float(m.group(4)), "並列サブ MIP " + m.group(1), m.group(2)))
        if "MIP: after root heuristics" in s:
            h = sorted(heur, reverse=True)[:5]; heur = []
            ev.append((t, "根のヒューリスティクス終わり: 長いもの " + "、".join(f"{n} {x:.1f} 秒 ({st})" for x, n, st in h)))
        m = re.search(r"background cuts: .*waited ([0-9.]+)s", s)
        if m and "rounds" in s and ("cut rows" in s or "not used" in s): ev.append((t, f"裏のカット: 待ち {m.group(1)} 秒"))
        m = re.search(r"MIP: restart (\d+) \((\d+) of (\d+)", s)
        if m: ev.append((t, f"再スタート {m.group(1)} ({m.group(2)} 列を固定)"))
        m = re.search(r"root LP after the restart: (\w+), (\d+) iters \(([0-9.]+)s\)", s)
        if m: ev.append((t, f"再スタート後の根の LP {m.group(2)} 反復 (状態 {m.group(1)}、経過 {m.group(3)} 秒の時点)"))
        m = re.search(r"MIP: +([0-9.]+)s nodes +(\d+) ", s)
        if m and int(m.group(2)) >= 1 and not any("木の始まり" in e[1] for e in ev): ev.append((t, f"木の始まり (ノード {m.group(2)})"))
        if "MIP: finish start" in s: ev.append((t, s.split("MIP: ")[1]))
    print(f"## {fn}")
    for t, e in ev: print(f"  {t:8.1f}  {e}")
