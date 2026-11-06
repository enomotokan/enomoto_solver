"""batchv の集計 (python3 summv.py [名前の頭 (既定 v)] > summv.txt): 問題・設定ごとに、出発の頂点 8 通りの根の下界 (最小・中央値・最大・幅)、時間 (中央値・最悪)、
時間切れの数、根の終わりの時刻 (`after root heuristics`) の中央値、主のループの後の目的値。行ごとの詳細も出す。"""
import json, re, statistics, sys
from pathlib import Path
D = Path(__file__).resolve().parent
rows = {}
PFX = sys.argv[1] if len(sys.argv) > 1 else "v"  # 名前の頭 (2 回目の絞った形は v2)
for f in sorted((D / "runs").glob(f"{PFX}_*.out")):
    try:
        r = json.loads([l for l in f.read_text().splitlines() if l.startswith("{")][-1])
    except Exception:
        continue
    _, rest = f.stem.split("_", 1)
    p, c, st = rest.rsplit("_", 2)
    t = (D / "runs" / (f.stem + ".log")).read_text(errors="replace")
    m = re.findall(r"after root heuristics: .*\(([\d.]+)s\)", t)
    r["root_end"] = float(m[0]) if m else None
    m = re.findall(r"background cuts: \d+ rounds, \d+ cut rows \(was \d+\), obj (\S+) -> (\S+)", t)
    r["main_obj"] = float(m[-1][0]) if m else None
    r["win"] = "ipm" if "ipm_crossover won" in t else ("simplex" if "simplex won" in t else "-")
    rows.setdefault((p, c), []).append((st, r))
print("問題          設定  回  根の下界 (最小・中央値・最大・幅)               時間 (中央値・最悪)  時間切れ  根の終わり (中央値)")
for (p, c), v in sorted(rows.items()):
    rb = [r["root_bound"] for _, r in v if r.get("root_bound") is not None]
    ts = [r["time"] for _, r in v]
    re_ = [r["root_end"] for _, r in v if r["root_end"] is not None]
    print(f"{p:13s} {c:5s} {len(v):2d}  {min(rb):12.5g} {statistics.median(rb):12.5g} {max(rb):12.5g} {max(rb)-min(rb):10.4g}   "
          f"{statistics.median(ts):6.1f} {max(ts):6.1f}  {sum(r['status']=='time_limit' for _, r in v):3d}   {statistics.median(re_) if re_ else float('nan'):6.2f}")
print()
for (p, c), v in sorted(rows.items()):
    for st, r in sorted(v):
        print(f"{p:13s} {c:5s} {st:3s} {r['status']:10s} {r['time']:6.1f} 根 {r['root_bound']:.6g} 主のループの後 {r['main_obj']} 根の終わり {r['root_end']} 解 {r.get('objective')} 節 {r['nodes']} 勝ち {r['win']}")
