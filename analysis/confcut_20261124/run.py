"""CONFCUT の効きの計測: python analysis/confcut_20261124/run.py OUT.jsonl CFG[,CFG...] [種 ...] [--tl 秒] [--par N] [--probs a,b]
1 スレッドで解き、結果 (状態・目的値・下界・ノード・時間) と stderr の CONFCUT_DIAG の行 (再スタートの入れ子の分も足す) を 1 行ずつ足す。
同じ (問題, 設定, 種) の行があれば飛ばす。設定は CFGS。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(D))
from probs import PROBS  # noqa: E402
CFGS = {
    "def": {"ENOMOTO_MIP_CONFCUT_DIAG": "1"},
    "c": {"ENOMOTO_MIP_CONFCUT": "1"},
    "p": {"ENOMOTO_MIP_CONFCUT": "p"},
    "cp": {"ENOMOTO_MIP_CONFCUT": "1p"},
    "cn": {"ENOMOTO_MIP_CONFCUT": "1n"},
    "cpn": {"ENOMOTO_MIP_CONFCUT": "1pn"},
    "cq": {"ENOMOTO_MIP_CONFCUT": "1", "ENOMOTO_MIP_CONFCUT_DIAG": "2"},
    "t4": {"ENOMOTO_MIP_CONFCUT_DIAG": "1", "ENOMOTO_MIP_TREE4_PROOFCUT": "1"},
}
args = sys.argv[1:]
def opt(name, default):
    if name in args:
        i = args.index(name); v = args[i + 1]; del args[i:i + 2]; return v
    return default
TL = float(opt("--tl", "120")); PAR = int(opt("--par", "1")); probs = opt("--probs", ",".join(PROBS)).split(",")
OUT = Path(args[0]); cfgs = args[1].split(","); seeds = [int(s) for s in args[2:]] or [1, 2, 3]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
KEYS = ["nodes", "lp_iters", "cut_rows_total", "node_cut_secs", "leaves_inf", "leaves_bound", "conflicts_rows", "lit_conflicts", "proofs_live",
        "sep_calls", "cand_conf", "cand_proof", "in_lp_conf", "in_lp_proof", "n_rounds", "nodes_with", "pruned_with", "clq_checked", "clq_redundant"]


def one(job):
    p, cfg, seed = job
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg]); env["ENOMOTO_MIP_THREADS"] = "1"; env["ENOMOTO_T_MIP_SEED"] = str(seed)
    t0 = time.time()
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                         "--time-limit", str(TL), "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=TL + 600)
    try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": pr.returncode, "err": pr.stderr[-1500:]}  # noqa: BLE001
    diag = {}
    for l in pr.stderr.splitlines():
        if l.startswith("CONFCUT_DIAG:"):
            nums = re.findall(r"[-0-9.e+]+(?=\s|$)", l.replace("|", " "))
            vals = [float(x) for x in re.findall(r" (-?[0-9][0-9.e+-]*)", l)]
            for k, v in zip(KEYS, vals):
                diag[k] = diag.get(k, 0) + v
            diag["lines"] = diag.get("lines", 0) + 1
    res["diag"] = diag
    res["warn"] = pr.stderr.count("MIP: WARNING")
    return {"problem": p, "cfg": cfg, "seed": seed, "tl": TL, "result": res, "wall": round(time.time() - t0, 1)}


jobs = [(p, c, s) for s in seeds for p in probs for c in cfgs if (p, c, s) not in done]
with ThreadPoolExecutor(PAR) as ex:
    for r in ex.map(one, jobs):
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        res = r["result"]; print(r["problem"], r["cfg"], r["seed"], res.get("status"), res.get("time"), res.get("nodes"), res["diag"].get("in_lp_conf"), res["diag"].get("in_lp_proof"), flush=True)
