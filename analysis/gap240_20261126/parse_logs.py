"""数取りの回 (measure.sh) のログから、前処理の後の大きさ・根・ノード・LP の反復・最初の解を抜き出す: python parse_logs.py > logs.txt (logs.json も書く)
- 当方 (logs/ours_<問題>_t1.log): 最後の「presolve: rows」の行 (前処理の後)、根の LP の値、cut loop done の後の値、木に入った時刻 (TRSTAT tree start)、
  木の LP の反復 (basis nb diag の tree first + others)、ノード、最初の解の時刻。
- SCIP (logs/scip_<問題>_def.stat、scipab.jsonl): 前処理の後の列・行、最初の LP、根の最後の下界 (Final Dual Bound)、走り (再スタート)、ノード、LP の反復。
- HiGHS (logs/highs_<問題>_t4.log): 前処理の後の行・列、根の再スタートの回数とその後の大きさ、ノード、LP の反復 (うち分離・ヒューリスティクス)。"""
import glob, json, re
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / "logs"
out = {}


def num(s):
    try: return float(s)
    except (TypeError, ValueError): return None


def last_json(text):
    for l in reversed(text.splitlines()):
        if l.startswith("{"):
            try: return json.loads(l)
            except ValueError: return None
    return None


for f in sorted(L.glob("ours_*_t1.log")):
    p = f.name[len("ours_"):-len("_t1.log")]
    t = f.read_text(errors="replace")
    r = last_json(t) or {}
    d = {"status": r.get("status"), "time": r.get("time"), "nodes": r.get("nodes"), "root_bound": r.get("root_bound"),
         "first_sol": r.get("first_solution_time"), "obj": r.get("objective"), "bound": r.get("bound")}
    pre = re.findall(r"presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", t)
    if pre:
        d["rows0"], d["cols0"], d["nz0"] = int(pre[0][0]), int(pre[0][2]), int(pre[0][4])
        d["rows"], d["cols"], d["nz"] = int(pre[-1][1]), int(pre[-1][3]), int(pre[-1][5])
    m = re.search(r"MIP: root LP ([-+0-9.eE]+) \((\d+) iters, ([0-9.]+)s\)", t)
    if m: d["root_lp"], d["root_lp_iters"], d["root_lp_s"] = num(m.group(1)), int(m.group(2)), num(m.group(3))
    m = re.findall(r"cut loop done: .*obj ([-+0-9.eE]+) -> ([-+0-9.eE]+)", t)
    if m: d["cut_loop_end"] = num(m[-1][1])
    m = re.findall(r"TRSTAT: tree start outer ([0-9.]+) .*restarts (\d+)", t)
    if m: d["tree_start"], d["restarts"] = num(m[-1][0]), int(m[-1][1])
    m = re.search(r"tree first LPs restored (\d+) iters (\d+) .*others (\d+) iters (\d+)", t)
    if m:
        d["tree_lps"] = int(m.group(1)) + int(m.group(3)); d["tree_iters"] = int(m.group(2)) + int(m.group(4))
    if d.get("tree_start") is not None and d.get("time") and d.get("nodes"):
        d["s_per_node"] = (d["time"] - d["tree_start"]) / max(d["nodes"], 1)
    out.setdefault(p, {})["ours_t1"] = d

SC = {}
if (D / "scipab.jsonl").exists():
    for l in open(D / "scipab.jsonl"):
        r = json.loads(l); SC[r["problem"]] = r
for f in sorted(L.glob("scip_*_def.stat")):
    p = f.name[len("scip_"):-len("_def.stat")]
    t = f.read_text(errors="replace")
    d = dict((k, SC.get(p, {}).get(k)) for k in ("status", "time", "nodes", "lpiters", "nruns", "root_bound"))
    # 前処理の後の問題 (Presolved Problem の節の Variables・Constraints)
    m = re.search(r"Presolved Problem\s*:.*?Variables\s*:\s*(\d+).*?Constraints\s*:\s*(\d+)", t, re.S)
    if m: d["cols"], d["rows"] = int(m.group(1)), int(m.group(2))
    m = re.search(r"Original Problem\s*:.*?Variables\s*:\s*(\d+).*?Constraints\s*:\s*(\d+)", t, re.S)
    if m: d["cols0"], d["rows0"] = int(m.group(1)), int(m.group(2))
    m = re.search(r"First LP value\s*:\s*([-+0-9.eE]+)", t)
    if m: d["root_lp"] = num(m.group(1))
    m = re.search(r"Final Dual Bound\s*:\s*([-+0-9.eE]+)", t)
    if m: d["root_final"] = num(m.group(1))
    m = re.search(r"First Solution\s*:\s*([-+0-9.eE]+)\s*\(in run (\d+), after (\d+) nodes, ([0-9.]+) seconds", t)
    if m: d["first_sol"] = num(m.group(4))
    if d.get("time") and d.get("nodes"): d["s_per_node"] = d["time"] / max(d["nodes"], 1)
    out.setdefault(p, {})["scip"] = d

for f in sorted(L.glob("highs_*_t4.log")):
    p = f.name[len("highs_"):-len("_t4.log")]
    t = f.read_text(errors="replace")
    r = last_json(t) or {}
    d = {"status": r.get("status"), "time": r.get("time"), "nodes": r.get("nodes"), "obj": r.get("objective"), "bound": r.get("bound")}
    m = re.search(r"Presolve reductions: rows (\d+)\(-?\d+\); columns (\d+)\(-?\d+\)", t)
    if m: d["rows"], d["cols"] = int(m.group(1)), int(m.group(2))
    rs = re.findall(r"Model after restart has (\d+) rows, (\d+) cols", t)
    d["restarts"] = len(rs)
    if rs: d["rows_after_restart"], d["cols_after_restart"] = int(rs[-1][0]), int(rs[-1][1])
    m = re.search(r"LP iterations\s+(\d+)", t)
    if m: d["lpiters"] = int(m.group(1))
    m = re.search(r"^\s+(\d+) \(separation\)", t, re.M)
    if m: d["lpiters_sep"] = int(m.group(1))
    m = re.search(r"^\s+(\d+) \(heuristics\)", t, re.M)
    if m: d["lpiters_heur"] = int(m.group(1))
    if d.get("time") and d.get("nodes"): d["s_per_node"] = d["time"] / max(d["nodes"], 1)
    out.setdefault(p, {})["highs_t4"] = d

json.dump(out, open(D / "logs.json", "w"), ensure_ascii=False, indent=1)


def g(x, k, fmt="{:.4g}"):
    v = x.get(k) if x else None
    return "—" if v is None else (fmt.format(v) if isinstance(v, float) else str(v))


print("| 問題 | 当方 t1: 前処理後 行×列 | 根 LP → カット後 | 木に入る (秒) | ノード | 秒/ノード | 反復/LP | 最初の解 (秒) | SCIP: 前処理後 列×行 | 根 LP → 根の最後 | 走り | ノード | 秒/ノード | HiGHS t4: 前処理後 行×列 | 再スタート (後の列) | ノード | 秒/ノード |")
print("|---|---|---|---:|---:|---:|---:|---:|---|---|---:|---:|---:|---|---|---:|---:|")
for p in sorted(out):
    o, s, h = out[p].get("ours_t1"), out[p].get("scip"), out[p].get("highs_t4")
    ipl = f"{o['tree_iters'] / max(o['tree_lps'], 1):.0f}" if o and o.get("tree_lps") else "—"
    print(f"| {p} | {g(o, 'rows')}×{g(o, 'cols')} | {g(o, 'root_lp')} → {g(o, 'cut_loop_end')} | {g(o, 'tree_start', '{:.0f}')} | {g(o, 'nodes')} | {g(o, 's_per_node', '{:.3f}')} | {ipl} | {g(o, 'first_sol', '{:.1f}')} "
          f"| {g(s, 'cols')}×{g(s, 'rows')} | {g(s, 'root_lp')} → {g(s, 'root_final')} | {g(s, 'nruns')} | {g(s, 'nodes')} | {g(s, 's_per_node', '{:.3f}')} "
          f"| {g(h, 'rows')}×{g(h, 'cols')} | {g(h, 'restarts')} ({g(h, 'cols_after_restart')}) | {g(h, 'nodes')} | {g(h, 's_per_node', '{:.3f}')} |")
