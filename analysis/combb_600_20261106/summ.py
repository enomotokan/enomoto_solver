"""res_<秒>.jsonl とログ (logs/) の集計 (python3 summ.py [秒] > summ_<秒>.md)。問題・設定・回ごとに 状態・時間・最後のギャップ・
根の LP の時間 (LP probing の根の LP の行、なければ `root LP` の行の秒)・根の終わり (`after root heuristics` の時刻、最初と最後 (再スタートの後))・再スタートの後の根の LP の秒の和・ノード数。
前の 600 秒 (analysis/mitt600_20261103 の res2_s*.jsonl = 既定、res_s*.jsonl = HiGHS と ours (旧))・cfg_def_* も並べる。"""
import glob, gzip, json, math, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
M = D.parent / "mitt600_20261103"
TL = int(sys.argv[1]) if len(sys.argv) > 1 else 600


def gap(r):
    z, b = r.get("objective"), r.get("bound")
    if r.get("status") in ("optimal", "infeasible", "Optimal"):
        return 0.0
    if z is None or b is None or not (math.isfinite(z) and math.isfinite(b)):
        return 1.0
    return min(1.0, abs(z - b) / max(abs(z), abs(b), 1e-9)) if z != b else 0.0


def loginfo(p, c, rep):
    fn = D / "logs" / f"{p}_{c}_r{rep}_{TL}.log.gz"
    if not fn.exists():
        return {}
    t = gzip.open(fn, "rt", errors="replace").read()
    m = re.findall(r"after root heuristics: .*\(([\d.]+)s\)", t)
    lpp = re.findall(r"LP probing: root LP ([\d.]+)s \((\d+) iters", t)
    rl = re.findall(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", t)
    sol = re.findall(r"MIP: solving the root LP \(([\d.]+)s\)", t)
    # 再スタートごとに時計が 0 に戻るので、最後の根の終わりの通しの時刻 = それより前の再スタートの時刻の和 + 最後の `after root heuristics`
    last_ar = [mm.end() for mm in re.finditer(r"after root heuristics: .*\(([\d.]+)s\)", t)]
    rst = [(mm.end(), float(mm.group(1))) for mm in re.finditer(r"MIP: restart \d+ \(.*\) \(([\d.]+)s\)", t)]
    abs_last = (sum(x for pos, x in rst if pos < last_ar[-1]) + float(m[-1])) if m else None
    rr = re.findall(r"root LP after the restart: \S+, \d+ iters \(([\d.]+)s\)", t)
    return {"root_end": float(m[0]) if m else None, "root_end_last": abs_last, "n_restart": len(rr), "rr_lp": sum(map(float, rr)), "lpprobe": float(lpp[0][0]) if lpp else None,
            "rootlp_at": float(rl[0][2]) if rl else None, "rootlp_start": float(sol[0]) if sol else None, "rootlp_obj": rl[0][0] if rl else None}


fmt = lambda x, d=1: "-" if x is None else f"{x:.{d}f}"
R = defaultdict(list)
for l in open(D / f"res_{TL}.jsonl"):
    d = json.loads(l)
    R[(d["problem"], d["cfg"])].append((d["rep"], d["result"]))
prev = defaultdict(list)
for f in sorted(glob.glob(str(M / "res2_s*.jsonl"))) + sorted(glob.glob(str(M / "res_s*.jsonl"))) + sorted(glob.glob(str(M / "cfg_def_*.jsonl"))):
    tag = "旧既定 (res2)" if "res2_" in f else ("cfg_def" if "cfg_def" in f else "res")
    for l in open(f):
        d = json.loads(l)
        prev[d["problem"]].append((tag if tag != "res" else ("HiGHS" if d["solver"] == "highs" else "旧 ours (res)"), d["result"]))
probs = list(dict.fromkeys(p for p, _ in R))
print(f"| 問題 | 設定 | 回 | 状態 | 時間 | 最後のギャップ | 主 | 双対の限界 | LP probing の根の LP 秒 | 根の LP の終わり | 根の終わり (最初 / 最後、通しの秒) | 再スタート後の根の LP 秒 (回数) | ノード |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for p in probs:
    for c in ("def", "combb"):
        for rep, r in sorted(R.get((p, c), []), key=lambda x: x[0]):
            li = loginfo(p, c, rep)
            st = "解" if r.get("status") in ("optimal", "infeasible") else ("時間切れ" if r.get("status") == "time_limit" else r.get("status"))
            print(f"| {p} | {c} | {rep} | {st} | {fmt(r.get('time'))} | {100 * gap(r):.3f}% | {r.get('objective')} | {r.get('bound')} | {fmt(li.get('lpprobe'), 2)} | {fmt(li.get('rootlp_at'), 2)} | {fmt(li.get('root_end'))} / {fmt(li.get('root_end_last'))} | {fmt(li.get('rr_lp'))} ({li.get('n_restart')}) | {r.get('nodes')} |")
    for tag, r in prev.get(p, []):
        st = r.get("status")
        print(f"| {p} | {tag} | | {st} | {fmt(r.get('time'))} | {100 * gap(r):.3f}% | {r.get('objective')} | {r.get('bound')} | | | | | {r.get('nodes')} |")

# 短い表: 問題ごとに 既定 / combb の (回ごとの) 状態・最後のギャップ・根の終わり (通し)
print()
print("| 問題 | 既定: ギャップ (根の終わり 秒) | combb: ギャップ (根の終わり 秒) | 旧既定 res2 | HiGHS |")
print("|---|---|---|---|---|")
for p in probs:
    cells = []
    for c in ("def", "combb"):
        xs = []
        for rep, r in sorted(R.get((p, c), []), key=lambda x: x[0]):
            li = loginfo(p, c, rep)
            g = "解 %.0f 秒" % r["time"] if r.get("status") in ("optimal", "infeasible") else f"{100 * gap(r):.3f}%"
            xs.append(f"{g} ({fmt(li.get('root_end_last'), 0)})")
        cells.append(" / ".join(xs))
    pv = {t: r for t, r in prev.get(p, [])}
    hs = pv.get("HiGHS")
    h = "-" if hs is None else ("解 %.0f 秒" % hs["time"] if hs.get("status") == "Optimal" else f"{100 * gap(hs):.2f}%")
    o = pv.get("旧既定 (res2)")
    print(f"| {p} | {cells[0]} | {cells[1]} | {'-' if o is None else f'{100 * gap(o):.2f}%'} | {h} |")
