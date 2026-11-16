"""調べ B の集計: python summB.py > summ.txt。問題ごとに 4 設定の状態・秒・節点数・根の下界 (ours は結果の root_bound、HiGHS はログの
節点 0 の最後の行) と、木の中の下界の上がり方: ギャップ g = (最適値 − 下界) / |最適値| が根のギャップの 1/2・1/4・1/10・1/100 に
なった節点数と秒 (ours はログの「MIP: 秒 nodes N open M lb L ub U」、HiGHS は B&B の表の行)。"""
import gzip, json, glob, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from probsB import PROBS, CFGS  # noqa: E402
OURS = re.compile(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+)")
HIG = re.compile(r"^\s*[A-Za-z]?\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)s")
def num(s):
    try: return float(s)
    except ValueError: return float("-inf") if s.startswith("-") else float("inf")
def traj(p, c):
    f = D / "logs" / f"{p}_{c}.log.gz"
    if not f.exists(): return [], None
    pts, root = [], None
    for l in gzip.open(f, "rt", errors="replace"):
        if c.startswith("highs"):
            m = HIG.match(l)
            if m:
                n, lb, t = int(m.group(1)), num(m.group(5)), float(m.group(12))
                if n == 0: root = lb
                pts.append((n, lb, t))
        else:
            m = OURS.search(l)
            if m: pts.append((int(m.group(2)), num(m.group(4)), float(m.group(1))))
    return pts, root
R = {}
for f in glob.glob(str(D / "res_*.jsonl")):
    for l in open(f):
        r = json.loads(l); R[(r["problem"], r["cfg"])] = r["result"]
for p, opt, integ in PROBS:
    print(f"\n## {p} (最適 {opt})")
    for c in CFGS:
        x = R.get((p, c))
        if not x: print(f"  {c:10s} -"); continue
        pts, hroot = traj(p, c)
        root = x.get("root_bound") if not c.startswith("highs") else hroot
        g0 = (opt - root) / abs(opt) if root is not None and abs(root) < 1e300 else None
        marks = []
        if g0 and g0 > 0:
            for fr in (2, 4, 10, 100):
                hit = next(((n, t) for n, lb, t in pts if abs(lb) < 1e300 and (opt - lb) / abs(opt) <= g0 / fr), None)
                marks.append(f"1/{fr}: " + (f"{hit[0]}節点 {hit[1]:.0f}s" if hit else "-"))
        last = pts[-1] if pts else None
        print(f"  {c:10s} {str(x.get('status'))[:9]:9s} {x.get('time', 0):6.1f}s 節点 {x.get('nodes')} 根の下界 {root} (ギャップ {g0:.3e})" if g0 is not None else f"  {c:10s} {str(x.get('status'))[:9]:9s} {x.get('time', 0):6.1f}s 節点 {x.get('nodes')} 根の下界 {root}",
              ("| " + "、".join(marks)) if marks else "", f"| 最後の行 {last}" if last else "")
