"""詳細ログ (logs/*.err) から、求解 (再スタートで区切る) ごとに根の LP・カットのループの後・背景カットの後の値、再スタートの時点の
木の下界 (最後の経過の行の lb)、新たな固定、前処理の縮みを出す。使い方: python summ_log.py logs/X.err ..."""
import re, sys
for f in sys.argv[1:]:
    print("==", f)
    seg, last_lb, t0 = 0, None, 0.0
    for l in open(f, errors="replace"):
        if m := re.match(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+)", l):
            print(f"  [{seg}] presolve rows {m[1]}->{m[2]} cols {m[3]}->{m[4]}")
        elif m := re.match(r"MIP: root LP ([-0-9.e+]+) \(", l):
            print(f"  [{seg}] root LP {float(m[1]):.6g}")
        elif m := re.match(r"MIP: cut loop done: .*obj ([-0-9.e+]+) -> ([-0-9.e+]+)", l):
            print(f"  [{seg}] cut loop -> {float(m[2]):.6g}  ({l.split(':',2)[2].strip()[:60]})")
        elif m := re.match(r"MIP: background cuts: \d+ rounds, .*obj ([-0-9.e+]+) -> ([-0-9.e+]+)", l):
            print(f"  [{seg}] bg cuts -> {float(m[2]):.6g}")
        elif m := re.match(r"MIP: +([0-9.]+)s nodes +(\d+) .* lb ([-0-9.e+inf]+) ub ([-0-9.e+inf]+)", l):
            last_lb = (float(m[1]), int(m[2]), m[3], m[4])
        elif m := re.match(r"MIP: restart (\d+) \((\d+) of (\d+) integer columns newly fixed.*\(([0-9.]+)s\)", l):
            print(f"  [{seg}] last progress (t, nodes, lb, ub) {last_lb}")
            print(f"  RESTART {m[1]}: newly fixed {m[2]}/{m[3]} at {m[4]}s in this solve")
            seg += 1; last_lb = None
        elif "RESTART_ROOT_CUTS" in l or "TREE1 restart" in l or "cuts from before the restart" in l:
            print("   ", l.strip()[5:150])
    print(f"  [{seg}] last progress {last_lb}")
