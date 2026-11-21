"""潜りの DSE の作り直しを手間で省く試作 ENOMOTO_MIP_DSE_COST の根だけの比べ (P2-INF、2026-11-21):
python run_root.py 出力名 秒 並べる数 [問題...]  (問題を省くと P の 58 問)
根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1・決定的な設定 (ENOMOTO_MIP_TIMEDEP=b・ENOMOTO_MIP_DET1T=onil) で
def と dc (ENOMOTO_MIP_DSE_COST=1) を回し、根の発見法の潜りの秒・木に入るまでの秒・根の下界・解を <出力名>.jsonl に足す (途中から再開できる)。
潜りと DSE_COST の行は logs_<出力名>/<問題>_<設定>.txt に残す。
RR_TREE=1 なら根だけにせず木まで解く (節の数・時間も残す)。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
name, tl, par = sys.argv[1], sys.argv[2], int(sys.argv[3])
probs = sys.argv[4:] or sorted({t[0] for s in json.loads((WT / "analysis/vsscip_20261120/shards_P.json").read_text())["shards"] for t in s})
CF = {"def": {}, "dc": {"ENOMOTO_MIP_DSE_COST": "1"}}
out = D / f"{name}.jsonl"; LOGD = D / f"logs_{name}"; LOGD.mkdir(exist_ok=True)
done = {(json.loads(l)["problem"], json.loads(l)["cfg"]) for l in open(out)} if out.exists() else set()
ENV = {"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_TIMEDEP": "b", "ENOMOTO_MIP_DET1T": "onil"}
if os.environ.get("RR_TREE") == "1": del ENV["ENOMOTO_MIP_NODE_LIMIT"]
RD = re.compile(r"MIP: root heuristic (.*diving.*?): (.*) \(([0-9.]+)s\)$")
RC = re.compile(r"MIP: DSE_COST skipped (\d+) built (\d+) built-without-estimate (\d+)")
RT = re.compile(r"TRSTAT: tree start outer (\S+) solve (\S+)")
def run(pc):
    p, c = pc
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}; env.update(ENV); env.update(CF[c])
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) + 300)
        err, so = r.stderr, r.stdout
    except subprocess.TimeoutExpired as e:
        err = e.stderr.decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); so = ""
    ls = err.splitlines()
    dives = [(m.group(1), m.group(2), float(m.group(3))) for l in ls if (m := RD.search(l))]
    dc = next(([int(x) for x in m.groups()] for l in ls if (m := RC.search(l))), None)
    tr = next(((float(m.group(1)), float(m.group(2))) for l in ls if (m := RT.search(l))), None)
    (LOGD / f"{p}_{c}.txt").write_text("\n".join(l for l in ls if RD.search(l) or RC.search(l) or RT.search(l) or "restart " in l[:20]) + "\n")
    res = {}
    try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash"}  # noqa: BLE001
    return {"problem": p, "cfg": c, "status": res.get("status"), "root_bound": res.get("root_bound"), "objective": res.get("objective"), "time": res.get("time"), "nodes": res.get("nodes"), "bound": res.get("bound"),
            "dives": dives, "dive_secs": round(sum(d[2] for d in dives), 3), "dse_cost": dc, "tree_start": tr, "wall": round(time.time() - t0, 1)}
todo = [(p, c) for p in probs for c in CF if (p, c) not in done]
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, todo):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["problem"], row["cfg"], row["status"], row["dive_secs"], row["dse_cost"], row["tree_start"], row["wall"], flush=True)
