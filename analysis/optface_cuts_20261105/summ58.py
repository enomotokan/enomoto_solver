"""bench58_s*.jsonl の集計 (python3 summ58.py > summ58.txt): 種ごとの解けた数・シフト 1 の幾何平均・主のギャップの平均、試作 (par =
ENOMOTO_MIP_OPTFACE_PARCUTS=3) が働いた問題 (ログ logs/<問題>_par_s<種>.log.gz の `optface parcuts` の行) と採った写し、根の終わりの時刻
(`after root heuristics`) の比 (試作 / 既定)、時間・ギャップが大きく違った問題。"""
import gzip, json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
sg = lambda ts: math.exp(sum(math.log(t + 1) for t in ts) / len(ts)) - 1


def gap(r):
    z, b = r.get("objective"), r.get("bound")
    if r.get("status") in ("optimal", "infeasible"):
        return 0.0
    if z is None or b is None or not math.isfinite(b):
        return 1.0
    return min(1.0, abs(z - b) / max(abs(z), abs(b), 1e-9)) if z != b else 0.0


# 引数 (省略可): 結果の名前の頭 (既定 bench58)・試作の設定名 (既定 par)・ログのフォルダー (既定 logs)。
# 例: python3 summ58.py bench58sgl sgl logs_bench58sgl_s1 > summ58sgl.txt
NAME = sys.argv[1] if len(sys.argv) > 1 else "bench58"
CFG = sys.argv[2] if len(sys.argv) > 2 else "par"
LOGDIR = sys.argv[3] if len(sys.argv) > 3 else "logs"


def loginfo(p, c, s):
    fn = D / LOGDIR / f"{p}_{c}_s{s}.log.gz"
    if not fn.exists():
        return {}
    t = gzip.open(fn, "rt", errors="replace").read()
    m = re.findall(r"after root heuristics: .*\(([\d.]+)s\)", t)
    info = {"root_end": float(m[0]) if m else None}
    info["copies"] = re.findall(r"optface parcuts: (\d+) background copies \((\d+) from", t)
    info["use"] = re.findall(r"optface parcuts: use copy (\d+)", t)
    info["notdeg"] = len(re.findall(r"optface parcuts: \d+ candidates \(<", t))
    info["replaced"] = re.findall(r"background cuts: \d+ rounds, \d+ cut rows \(was \d+\), obj (\S+) -> (\S+)", t)
    info["rb"] = re.findall(r"root LP both|optface for the background cuts: .*\(move ([\d.]+)s, all ([\d.]+)s\)", t)
    return info


for s in (1, 2):
    f = D / f"{NAME}_s{s}.jsonl"
    if not f.exists():
        continue
    R = {}
    for l in open(f):
        d = json.loads(l)
        R[(d["problem"], d["cfg"])] = d["result"]
    probs = sorted({p for p, _ in R})
    both = [p for p in probs if (p, "def") in R and (p, CFG) in R]
    print(f"== 種 {s}: 両方そろった問題 {len(both)}")
    for c in ("def", CFG):
        rs = [R[(p, c)] for p in both]
        solved = sum(r.get("status") in ("optimal", "infeasible") for r in rs)
        ts = [min(60.0, r.get("time", 60.0)) if r.get("status") in ("optimal", "infeasible") else 60.0 for r in rs]
        print(f"  {c:4s} 解けた {solved:2d}  SGM {sg(ts):6.2f}  主ギャップの平均 {100 * sum(gap(r) for r in rs) / len(rs):5.2f}%")
    worked, ratios, movesec = [], [], []
    for p in both:
        a, b = loginfo(p, "def", s), loginfo(p, CFG, s)
        if a.get("root_end") and b.get("root_end"):
            ratios.append(b["root_end"] / max(a["root_end"], 1e-3))
        movesec += [float(x[1]) for x in b.get("rb", []) if x]
        if b.get("copies") and any(int(k[1]) > 0 for k in b["copies"]):
            worked.append((p, b))
    if ratios:
        rs = sorted(ratios)
        gm = math.exp(sum(math.log(x) for x in rs) / len(rs))
        print(f"  根の終わりの時刻の比 ({CFG} / def, {len(rs)} 問): 中央値 {rs[len(rs) // 2]:.3f}  幾何平均 {gm:.3f}  最大 {rs[-1]:.2f}")
    if movesec:
        print(f"  移動の時間: {len(movesec)} 回、合計 {sum(movesec):.2f} 秒、最大 {max(movesec):.3f} 秒")
    print(f"  別の基底の写しを立てた問題 {len(worked)}:")
    for p, b in worked:
        ra, rb = R[(p, "def")], R[(p, CFG)]
        uses = ",".join(b["use"]) if b["use"] else "-"
        rep = b["replaced"][-1] if b["replaced"] else None
        print(f"    {p:28s} def {ra.get('status')[:8]:8s} {ra.get('time', 0):6.1f}s 根 {ra.get('root_bound')!s:.12s}  {CFG} {rb.get("status")[:8]:8s} {rb.get("time", 0):6.1f}s 根 {rb.get('root_bound')!s:.12s}  一番高い写し {uses}  置き換え {rep}")
    print("  時間・ギャップが大きく違った問題 (|差| > 20% かつ 2 秒以上、または片方だけ解けた):")
    for p in both:
        ra, rb = R[(p, "def")], R[(p, CFG)]
        sa, sb = ra.get("status") in ("optimal", "infeasible"), rb.get("status") in ("optimal", "infeasible")
        ta, tb = ra.get("time", 60), rb.get("time", 60)
        if sa != sb or (sa and abs(ta - tb) > max(2.0, 0.2 * min(ta, tb))) or abs(gap(ra) - gap(rb)) > 0.01:
            print(f"    {p:28s} def {ra.get('status')[:8]:8s} {ta:6.1f}s gap {100 * gap(ra):6.2f}%   {CFG} {rb.get("status")[:8]:8s} {tb:6.1f}s gap {100 * gap(rb):6.2f}%")
