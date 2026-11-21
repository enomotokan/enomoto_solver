"""SCIP の統計 (logs/<問題>_scip.stat) と HiGHS のログ (logs/<問題>_highs.log) から根の段の時間を拾う。
使い方: python othersum.py 問題...   SCIP は limits/nodes=1 (根で止める)、HiGHS は mip_max_nodes=1 の回。"""
import re, sys
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
def table(L, head):
    i = next((k for k, l in enumerate(L) if l.startswith(head)), None)
    if i is None: return {}
    out = {}
    for l in L[i + 1:]:
        if not l.startswith("  "): break
        f = l.split(":")
        if len(f) < 2: continue
        v = f[1].split()
        try: out[f[0].strip()] = [float(x) if x not in "-" else 0.0 for x in v]
        except ValueError: pass
    return out
for p in sys.argv[1:]:
    s = D / f"{p}_scip.stat"
    if s.exists():
        L = s.read_text(errors="replace").splitlines()
        g = lambda k: next((l.split(":", 1)[1].split()[0] for l in L if l.startswith(k)), "?")
        lp = table(L, "LP  "); sep = table(L, "Separators"); he = table(L, "Primal Heuristics"); pr = table(L, "Presolvers")
        hs = sorted(((v[0], k) for k, v in he.items() if v and v[0] >= 1), reverse=True)[:5]
        ss = sum(v[0] for k, v in sep.items() if v and k != "cut pool")
        print(f"## {p} SCIP: 全体 {g('  solving')} 秒、前処理 {g('  presolving')} 秒、最初の LP {g('  First LP Time')} 秒・{g('  First LP Iters')} 反復 (値 {g('  First LP value')})、"
              f"根の終わりの下界 {g('  Final Dual Bound')}、根の LP 反復 {g('  Final Root Iters')}")
        print(f"   LP: 双対 {lp.get('dual LP', [0])[0]} 秒・{lp.get('dual LP', [0, 0, 0])[2]:.0f} 反復・{lp.get('dual LP', [0])[1]:.0f} 回、潜り/プロービング {lp.get('diving/probing LP', [0])[0]} 秒・{lp.get('diving/probing LP', [0, 0, 0])[2]:.0f} 反復、強分枝 {lp.get('strong branching', [0])[0]} 秒")
        print(f"   分離の計 {ss:.1f} 秒、ヒューリスティクスの長いもの " + "、".join(f"{k} {x:.1f}" for x, k in hs))
    h = D / f"{p}_highs.log"
    if h.exists():
        L = h.read_text(errors="replace").splitlines()
        prog = []
        for l in L:
            m = re.match(r"^\s*(\S?)\s+(\d+)\s+(\d+)\s+(\d+)\s+[0-9.]+%\s+(\S+)\s+(\S+)\s+.*?\s(\d+)\s+([0-9.]+)s\s*$", l)
            if m: prog.append((m.group(1), float(m.group(8)), int(m.group(7)), m.group(5), int(m.group(2))))
        first = next((x for x in prog if x[2] > 0), None)
        rs = next((k for k, l in enumerate(L) if "restarting" in l), None)
        tim = [l.strip() for l in L if re.search(r"\((Presolve|Solve)\)|MIP +time|subMIP time|\(strong br\.\)|\(separation\)|\(heuristics\)|LP iterations|Timing", l)]
        print(f"## {p} HiGHS: 根の LP の終わり {first[1] if first else '?'} 秒 ({first[2] if first else '?'} 反復、値 {first[3] if first else '?'})、"
              f"再スタート {'あり' if rs else 'なし'}、最後の進みの行 {prog[-1][1] if prog else '?'} 秒 (下界 {prog[-1][3] if prog else '?'}、ノード {prog[-1][4] if prog else '?'})")
        print("   " + " | ".join(tim))
