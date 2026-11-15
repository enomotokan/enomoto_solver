"""highspy 1.15.1 で 1 問を解き、終わりの統計 (ノード・LP の反復の内訳) を jsonl に 1 行足す (2026-11-08)。
使い方: python hrun.py 出力.jsonl 設定 問題 秒   (設定は CFGS の名前。threads=4・mip_rel_gap=0)
ノードの LP の反復 = 全体 - 強分枝 - 分離 - ヒューリスティクス (HiGHS の終わりの表の数)。
計装した HiGHS を使うときは HIGHS_BIN を与えると、highspy の代わりにその実行形式で解き、NLP の行を logs/<問題>_<設定>.nlp.gz に置く。
"""
import gzip, json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
CFGS = {
    "def": {},
    "noperturb": {"dual_simplex_cost_perturbation_multiplier": 0.0},
    "age100": {"mip_lp_age_limit": 100},
    "age3": {"mip_lp_age_limit": 3},
    "noheur": {"mip_heuristic_effort": 0.0},
    "norestart": {"mip_allow_restart": False},
    "rel1": {"mip_pscost_minreliable": 1},
    "rel0": {"mip_pscost_minreliable": 0},
    "rel16": {"mip_pscost_minreliable": 16},
    "nosepnodes": {"mip_allow_cut_separation_at_nodes": False},
    "dsedev": {"simplex_dual_edge_weight_strategy": 1},
    "dantzig": {"simplex_dual_edge_weight_strategy": 0},
    "dseexact": {"simplex_dual_edge_weight_strategy": 2},
    "noscale": {"simplex_scale_strategy": 0},
    "pool1": {"mip_pool_age_limit": 1},
    "nopresolve": {"presolve": "off"},
    "noperturb_dev": {"dual_simplex_cost_perturbation_multiplier": 0.0, "simplex_dual_edge_weight_strategy": 1},
}
for k, v in list(os.environ.items()):
    if k.startswith("HCFG_"):  # HCFG_<名前>="opt=val,opt=val"
        d = {}
        for x in v.split(","):
            a, b = x.split("=", 1)
            try:
                b = int(b)
            except ValueError:
                try:
                    b = float(b)
                except ValueError:
                    b = {"true": True, "false": False}.get(b, b)
            d[a] = b
        CFGS[k[5:].lower()] = d
# 既知の最適値 + 余裕 (打ち切り値の比べ `cut*` に使う。MIPLIB の値)
CUTV = {"supportcase33": -344.5, "ns1830653": 20622.5, "rococoC10-001000": 11460.5, "traininstance6": 28290.5,
        "uccase12": 11507.42, "mzzv11": -21717.5, "tr12-30": 130596.5}
out, cfg, p, tl = sys.argv[1], sys.argv[2], sys.argv[3], float(sys.argv[4])
if cfg.startswith("cut"):
    CFGS[cfg] = dict(CFGS.get(cfg[3:] or "def", {}), objective_bound=CUTV[p])
mps = WT / ".miplib_cache" / f"{p}.mps"
opts = dict(threads=4, mip_rel_gap=0.0, time_limit=tl, **CFGS[cfg])
log = D / "logs" / f"{p}_h{cfg}.log"
t0 = time.time()
hb = os.environ.get("HIGHS_BIN")
if hb:
    with tempfile.NamedTemporaryFile("w", suffix=".opt", delete=False) as f:
        for k, v in opts.items():
            f.write(f"{k} = {str(v).lower() if isinstance(v, bool) else v}\n")
        of = f.name
    env = dict(os.environ, HIGHS_NODELP="1")
    with open(log, "w") as lf, gzip.open(D / "logs" / f"{p}_h{cfg}.nlp.gz", "wt") as nf:
        pr = subprocess.Popen([hb, "--options_file", of, str(mps)], stdout=lf, stderr=subprocess.PIPE, text=True, env=env)
        for l in pr.stderr:
            if l.startswith("NLP"):
                nf.write(l)
        pr.wait()
    txt = log.read_text()
else:
    import highspy
    h = highspy.Highs()
    h.setOptionValue("log_file", str(log))
    h.setOptionValue("output_flag", True)
    h.setOptionValue("log_to_console", False)
    for k, v in opts.items():
        h.setOptionValue(k, v)
    h.readModel(str(mps))
    h.run()
    txt = log.read_text()
wall = time.time() - t0
def g(pat, cast=float):
    m = re.search(pat, txt)
    return cast(m.group(1)) if m else None
rec = {"problem": p, "cfg": cfg, "tl": tl, "wall": round(wall, 1),
       "status": g(r"Status\s+(\S.*)", str), "primal": g(r"Primal bound\s+(\S+)"), "dual": g(r"Dual bound\s+(\S+)"),
       "nodes": g(r"Nodes\s+(\d+)", int), "lp_total": g(r"LP iterations\s+(\d+)", int),
       "lp_sb": g(r"(\d+) \(strong br\.\)", int), "lp_sep": g(r"(\d+) \(separation\)", int), "lp_heur": g(r"(\d+) \(heuristics\)", int),
       "time": g(r"Timing\s+(\S+)")}
try:
    nlp = rec["lp_total"] - rec["lp_sb"] - rec["lp_sep"] - rec["lp_heur"]
    rec["lp_node"] = nlp
    rec["it_per_node"] = round(nlp / max(rec["nodes"], 1), 1)
except TypeError:
    pass
with open(out, "a") as f:
    f.write(json.dumps(rec) + "\n")
print(json.dumps(rec), flush=True)
