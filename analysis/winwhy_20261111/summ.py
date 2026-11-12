"""WINWHY (hiloss_20261111/summ.py の写し): logs/<問題>.<highs|ours>.log.gz から、2 つのソルバーの同じ量を取り出して並べる。
使い方: python summ.py [問題 ...] > summ.txt   (問題を書かなければ logs/ にある全部)。summ.json にも書く。
時刻はどちらもログの最初の行からの経過秒 (行の頭の印。MPS の読み込みは含まない)。
取り出す量: 前処理後の行・列・非零・整数、前処理の終わりの時刻、根の LP の値と時刻、根の終わり (最後の再スタートの後、木に入る直前) の下界と時刻、
最初の暫定解の時刻と値、最後に暫定解が良くなった時刻、再スタートの回数、ノード数、木の時間あたりのノード数、終わりの下界・暫定解、
下界と暫定解の推移 (10・30・60・120・300・600 秒)。"""
import gzip, json, math, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
MARKS = (10, 30, 60, 120, 300, 600)
TAG = {}  # 切り分けの回を読むとき: TAG['ours'] = 'タグ名'
def num(s):
    s = s.strip()
    m = {"k": 1e3, "M": 1e6, "G": 1e9}
    if s and s[-1] in m: return float(s[:-1]) * m[s[-1]]
    try: return float(s)
    except ValueError: return math.nan
def lines(p, solver):
    f = D / "logs" / f"{p}.{TAG.get(solver, solver)}.log.gz"
    if not f.exists(): return None
    out = []
    for l in gzip.open(f, "rt", errors="replace"):
        try: out.append((float(l[:8]), l[9:].rstrip("\n")))
        except ValueError: pass
    t0 = out[0][0] if out else 0.0
    return [(t - t0, s) for t, s in out] if out else None
def traj(pts, final_t):
    """pts: [(時刻, 下界, 暫定解)] -> 各 MARKS の時点の (下界, 暫定解)"""
    res = {}
    for mk in MARKS:
        if mk > final_t + 1 and mk != MARKS[-1]: res[mk] = None; continue
        cur = None
        for t, lb, ub in pts:
            if t <= mk: cur = (lb, ub)
        res[mk] = cur
    return res
ROW = re.compile(r"^ ?([A-Za-z ])\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)%\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)\s+(\S+)s$")
def highs(p):
    L = lines(p, "highs")
    if not L: return None
    r = dict(restarts=0); pts = []; seen_tree = False; root_end = None; cur_restart_rows = []
    for i, (t, s) in enumerate(L):
        m = re.match(r"MIP\s+\S+ has (\d+) rows; (\d+) cols; (\d+) nonzeros; (\d+) integer", s)
        if m: r["orig"] = [int(x) for x in m.groups()]
        if s.startswith("Solving MIP model with:") and "pre" not in r:
            rows = int(L[i + 1][1].split()[0]); cs = L[i + 2][1]; nz = int(L[i + 3][1].split()[0])
            m = re.match(r"\s*(\d+) cols \((\d+) binary, (\d+) integer, (\d+) implied int., (\d+) continuous", cs)
            c, b, gi, im, co = (int(x) for x in m.groups())
            r["pre"] = [rows, c, nz, b + gi + im]; r["pre_t"] = t
        m = re.match(r"Model after restart has (\d+) rows, (\d+) cols \((\d+) bin., (\d+) int., (\d+) impl., (\d+) cont.*and (\d+) nonzeros", s)
        if m:
            g = [int(x) for x in m.groups()]; r["last_restart"] = [g[0], g[1], g[6], g[2] + g[3] + g[4]]; r["restarts"] += 1; r["last_restart_t"] = t
            if r.get("nodes", 0) > 0: r["tree_restarts"] = r.get("tree_restarts", 0) + 1  # 直前の表の行でノードが進んでいれば木の中の再スタート
        m = ROW.match(s)
        if m:
            src, proc, inq, leaves, expl, bb, bs, gap, cuts, inlp, confl, iters, tm = m.groups()
            proc = num(proc); bb = num(bb); bs = num(bs); iters = num(iters)
            pts.append((t, bb, bs))
            if "root_lp" not in r and iters > 0 and math.isfinite(bb): r["root_lp"] = bb; r["root_lp_t"] = t
            if math.isfinite(bs) and "first_sol" not in r: r["first_sol"] = bs; r["first_sol_t"] = t; r["first_src"] = src.strip()
            if math.isfinite(bs) and bs != r.get("_last"): r["_last"] = bs; r["last_sol_t"] = t
            if proc == 0: r["root_bound"] = bb; r["root_end_t"] = t; r["root_cuts_inlp"] = num(inlp)
            r["nodes"] = proc; r["end_bound"] = bb; r["end_sol"] = bs; r["end_t"] = t; r["lp_iters"] = iters
        m = re.match(r"\s+Status\s+(.*)", s)
        if m: r["status"] = m.group(1).strip()
        m = re.match(r"\s+Nodes\s+(\d+)$", s)
        if m: r["nodes"] = int(m.group(1))
        m = re.match(r"\s+Timing\s+(\S+)", s)
        if m: r["time"] = float(m.group(1))
        m = re.match(r"\s+(\S+) \((Presolve|strong br.|separation|heuristics)\)", s)
        if m: r["x_" + m.group(2)] = float(m.group(1))
    r.pop("_last", None)
    if r.get("nodes") and "lp_iters" in r:  # ノードの LP の反復 / ノード (全体から強分岐・分離・ヒューリスティクスを引く)
        r["node_iters_per_node"] = (r["lp_iters"] - r.get("x_strong br.", 0) - r.get("x_separation", 0) - r.get("x_heuristics", 0)) / r["nodes"]
    r["traj"] = traj(pts, r.get("end_t", 0))
    return r
def ours(p):
    L = lines(p, "ours")
    if not L: return None
    r = dict(restarts=0, tree_restarts=0); pts = []; base = 0.0; lb = math.nan; ub = math.inf
    for t, s in L:
        if s.startswith("MIP:   ") and not re.match(r"MIP:\s+[\d.]+s nodes", s) and "global fixed" not in s: continue  # ヒューリスティクス・サブ MIP の細目
        m = re.match(r"MIP: presolve: rows (\d+) -> (\d+), free columns (\d+) -> (\d+), nonzeros (\d+) -> (\d+)", s)
        if m:
            g = [int(x) for x in m.groups()]
            if "pre" not in r: r["orig"] = [g[0], g[2], g[4]]; r["pre"] = [g[1], g[3], g[5]]; r["pre_t"] = t
            else: r["last_restart"] = [g[1], g[3], g[5]]
        m = re.match(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", s)
        if m and "root_lp" not in r: r["root_lp"] = float(m.group(1)); r["root_lp_t"] = t; r["root_lp_secs"] = float(m.group(3)); r["root_lp_iters"] = int(m.group(2))
        m = re.match(r"MIP: cut loop done: (\d+) cuts added, (\d+) in LP, pool (\d+) cuts .*obj (\S+) -> (\S+)", s)
        if m:
            if "cut0" not in r: r["cut0"] = float(m.group(5)); r["cut0_t"] = t
            r["cut_last"] = float(m.group(5)); r["cut_last_t"] = t; r["cuts_inlp"] = int(m.group(2)); r["cut_loops"] = r.get("cut_loops", 0) + 1
        m = re.match(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]", s)
        if m:
            z = float(m.group(1))
            if "first_sol" not in r: r["first_sol"] = z; r["first_sol_t"] = t; r["first_src"] = m.group(4)
            if z < ub: ub = z; r["last_sol_t"] = t; r["last_src"] = m.group(4)
            pts.append((t, lb, ub))
        m = re.match(r"MIP: restart (\d+) \(", s)
        if m: r["restarts"] = int(m.group(1)); r["last_restart_t"] = t
        if s.startswith("MIP: tree restart:"): r["tree_restarts"] += 1
        m = re.match(r"TRSTAT: tree start outer (\S+) solve (\S+) restarts (\d+) lb (\S+) ub (\S+)", s)
        if m:
            # TRSTAT の lb は根の LP の値のままのことがある (再スタートなしのとき) ので、最後のカットのループの値と大きい方
            lb = max(float(m.group(4)), r.get("cut_last", -math.inf)); r["root_bound"] = lb; r["root_end_t"] = t; ub = min(ub, float(m.group(5))); pts.append((t, lb, ub))
            r.setdefault("first_tree_t", t); r.setdefault("first_tree_lb", lb)
        m = re.match(r"MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+(\d+) lb (\S+) ub (\S+) lp_iters (\d+) sb_iters (\d+) sb_lps (\d+) sb_secs (\S+) node_iters (\d+) \(first (\d+), lps (\d+)\) heur_iters (\d+) dive_iters (\d+) lp_rows (\d+) node_cut_secs (\S+)", s)
        if m and "root_end_t" in r:
            g = m.groups(); ub = min(ub, float(g[4]))
            if not r.get("has_glb"): lb = float(g[3])  # TREE4 の行 (全スレッドの下界) がないときだけ主スレッドの値を使う
            pts.append((t, lb, ub))
            r["nodes_last"] = int(g[1]); r["open"] = int(g[2]); r["sb_secs"] = float(g[8]); r["node_lps"] = int(g[11]); r["node_iters"] = int(g[9]); r["lp_rows"] = int(g[14]); r["node_cut_secs"] = float(g[15])
            r["sb_lps"] = int(g[7]); r["heur_iters"] = int(g[12]); r["lp_iters"] = int(g[5])
        m = re.match(r"TREE4: t \S+ tid 0 glb (\S+) ub (\S+)", s)
        if m: r["has_glb"] = True; lb = float(m.group(1)); ub = min(ub, float(m.group(2))); pts.append((t, lb, ub))
        m = re.match(r"MIP: solve returned (\S+) after ([\d.]+)s", s)
        if m: r["status"] = m.group(1); r["time"] = float(m.group(2)); r["end_t"] = t
        m = re.match(r"MIP: node diag: nodes (\d+) node LPs (\d+) mean rows (\S+) iters (\d+) \((\S+)/LP\) LP secs (\S+) sep rounds (\d+) added (\d+) node cut secs (\S+)", s)
        if m: r["diag_nodes"] = int(m.group(1)); r["diag_lp_secs"] = float(m.group(6)); r["diag_iters_per_lp"] = float(m.group(5)); r["diag_rows"] = float(m.group(3))
    r["end_bound"] = lb; r["end_sol"] = ub if math.isfinite(ub) else math.nan
    r.setdefault("end_t", L[-1][0])
    r["traj"] = traj(pts, r["end_t"])
    for l in reversed(L):
        if l[1].startswith("{"):
            j = json.loads(l[1]); r["nodes"] = j.get("nodes"); r["end_bound"] = j.get("bound"); r["end_sol"] = j.get("objective"); r["json_status"] = j.get("status"); break
    return r
def f(v, w=13):
    if v is None or (isinstance(v, float) and math.isnan(v)): return "-".rjust(w)
    if isinstance(v, float) and math.isinf(v): return "inf".rjust(w)
    return f"{v:.8g}".rjust(w)
def show(p, h, o):
    print(f"\n=== {p} ===")
    def row(name, a, b): print(f"  {name:34} HiGHS {a:>30}   ENOMOTO {b:>30}")
    g = lambda d, k: (d or {}).get(k)
    sz = lambda v: "-" if not v else "/".join(str(x) for x in v)
    row("元の大きさ 行/列/非零(/整数)", sz(g(h, "orig")), sz(g(o, "orig")))
    row("前処理後 行/列/非零(/整数)", sz(g(h, "pre")), sz(g(o, "pre")))
    row("前処理の終わり (秒)", f(g(h, "pre_t")), f(g(o, "pre_t")))
    row("根の LP の値", f(g(h, "root_lp")), f(g(o, "root_lp")))
    row("根の LP の時刻 (秒)", f(g(h, "root_lp_t")), f(g(o, "root_lp_t")))
    row("最初のカットのループの後の下界", "-", f(g(o, "cut0")))
    row("再スタートの回数 (根 / 木)", f"{g(h, 'restarts', ) - g(h, 'tree_restarts') if h and g(h, 'tree_restarts') else g(h, 'restarts')} / {g(h, 'tree_restarts') or 0}" if h else "-", f"{g(o, 'restarts')} / {g(o, 'tree_restarts')}" if o else "-")
    row("最後の再スタート後 行/列/非零", sz(g(h, "last_restart")), sz(g(o, "last_restart")))
    row("木に入る直前の下界", f(g(h, "root_bound")), f(g(o, "root_bound")))
    row("木に入る時刻 (秒)", f(g(h, "root_end_t")), f(g(o, "root_end_t")))
    row("最初の暫定解 値", f(g(h, "first_sol")), f(g(o, "first_sol")))
    row("最初の暫定解 時刻 (出どころ)", f"{f(g(h, 'first_sol_t'))} ({g(h, 'first_src')})", f"{f(g(o, 'first_sol_t'))} ({g(o, 'first_src')})")
    row("最後に暫定解が良くなった時刻", f(g(h, "last_sol_t")), f"{f(g(o, 'last_sol_t'))} ({g(o, 'last_src')})")
    row("終わり 状態", str(g(h, "status")), str(g(o, "status")))
    row("終わり 時刻 (秒)", f(g(h, "end_t")), f(g(o, "end_t")))
    row("終わり 下界", f(g(h, "end_bound")), f(g(o, "end_bound")))
    row("終わり 暫定解", f(g(h, "end_sol")), f(g(o, "end_sol")))
    row("ノード数", f(g(h, "nodes")), f(g(o, "nodes")))
    def rate(d):
        if not d or not d.get("nodes") or "root_end_t" not in d: return None
        dt = d["end_t"] - d["root_end_t"]
        return d["nodes"] / dt if dt > 1e-9 else None
    row("木のノード/秒", f(rate(h)), f(rate(o)))
    row("LP の反復の総数", f(g(h, "lp_iters")), f(g(o, "lp_iters")))
    if o: print(f"  ENOMOTO の木: 強分岐 {g(o, 'sb_secs')} 秒 ({g(o, 'sb_lps')} LP)、ノードのカット {g(o, 'node_cut_secs')} 秒、ノードの LP {g(o, 'diag_lp_secs')} 秒 ({g(o, 'diag_iters_per_lp')} 反復/LP、平均 {g(o, 'diag_rows')} 行)、開いたノード {g(o, 'open')}")
    if h and g(h, "node_iters_per_node") is not None: print(f"  HiGHS のノードの LP: {g(h, 'node_iters_per_node'):.1f} 反復/ノード (強分岐・分離・ヒューリスティクスを除く)")
    if h: print(f"  HiGHS の LP 反復の内訳: 強分岐 {g(h, 'x_strong br.')}、分離 {g(h, 'x_separation')}、ヒューリスティクス {g(h, 'x_heuristics')}")
    print("  推移 (秒: 下界 / 暫定解。ENOMOTO の下界は TREE4 の行がないログでは主スレッドの経過の行のもので、全スレッドの下界 (終わりの行) より高く出ることがある)")
    for mk in MARKS:
        a = (h or {}).get("traj", {}).get(mk); b = (o or {}).get("traj", {}).get(mk)
        sa = "終了後" if a is None else f"{f(a[0])} /{f(a[1])}"; sb = "終了後" if b is None else f"{f(b[0])} /{f(b[1])}"
        print(f"    {mk:4}: HiGHS {sa:>28}   ENOMOTO {sb:>28}")
if __name__ == "__main__":
    ps = sys.argv[1:] or sorted({x.name.split(".")[0] for x in (D / "logs").glob("*.log.gz")})
    allr = {}
    for p in ps:
        h, o = highs(p), ours(p); allr[p] = dict(highs=h, ours=o); show(p, h, o)
    if not sys.argv[1:]:
        (D / "summ.json").write_text(json.dumps(allr, indent=0, default=str).replace("NaN", "null").replace("Infinity", "1e999"))
