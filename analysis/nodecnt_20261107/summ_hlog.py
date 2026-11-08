"""HiGHS のログ (logs/H_<問題>_<設定>.log) の要約: 下界・暫定解の推移 (1/4 ごと)、待ち行列・衝突のプールの最大、
再スタートの回数、強分枝・分離・ヒューリスティクスの LP 反復。使い方: python summ_hlog.py <ログ>..."""
import re, sys
for fn in sys.argv[1:]:
    prog = []; restarts = 0; it = {}; nodes = None; sym = []
    txt = open(fn, errors="replace").read()
    # 同じ内容が 2 度出ることがある (log_file と画面) ので最初の Solving report まで
    txt = txt.split("Solving report")[0] + "Solving report" + txt.split("Solving report")[1] if "Solving report" in txt else txt
    for l in txt.splitlines():
        if l.startswith("Restarting search"):
            restarts += 1
        if "orbitope" in l or "generators" in l:
            sym.append(l.strip())
        m = re.match(r"\s*(\S)?\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)s", l)
        if m:
            prog.append(dict(src=m.group(1) or "", proc=int(m.group(2)), q=int(m.group(3)), leaves=int(m.group(4)), lb=m.group(6), ub=m.group(7),
                             cuts=int(m.group(9)), inlp=int(m.group(10)), confl=int(m.group(11)), t=float(m.group(13))))
        m = re.match(r"\s*Nodes\s+(\d+)", l)
        if m: nodes = int(m.group(1))
        m = re.match(r"\s+(\d+) \((strong br\.|separation|heuristics)\)", l)
        if m: it[m.group(2)] = int(m.group(1))
    print("==", fn)
    if prog:
        T = prog[-1]["t"]
        for q in (0.25, 0.5, 0.75, 1.0):
            r = min(prog, key=lambda p: abs(p["t"] - q * T))
            print(f"  t={r['t']:.0f} proc={r['proc']} queue={r['q']} leaves={r['leaves']} lb={r['lb']} ub={r['ub']} cuts={r['cuts']} inlp={r['inlp']} confl={r['confl']}")
        print(f"  max queue {max(p['q'] for p in prog)}, max confl {max(p['confl'] for p in prog)}, restarts {restarts}, nodes {nodes}, iters {it}")
    for s in sym[:2]:
        print("  ", s)
