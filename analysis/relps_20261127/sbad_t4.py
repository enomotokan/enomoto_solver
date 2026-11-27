"""SB_ADAPT の 4 スレッドの外れの切り分け: 既定 (def) と既定 + ENOMOTO_MIP_SB_ADAPT=1 (sbad) を、1・4 スレッド・種 1・60 秒で解き、
ENOMOTO_MIP_SBADAPT_DIAG=1 の SBADAPT_FEAT の行 (fa・fb・cpn・deg) と、終わりの強分枝の量 (branching: … sb LPs … iters …) をスレッドごとに拾う。
設定 *_np は ENOMOTO_MIP_PARSB=0 (強分枝の子の投機を切る) を足す (RUN_CFGS・RUN_TH で選ぶ)。sbad_nocw は衝突の重みの床を外し、sbad_nofb は f_B ≈ 0 (子の伝播も床も止まる)、sbad_nofa は f_A ≈ 0 (予算と退化のノードの強分枝が止まる)。
問題は combo2 の c2b / c2a の 4 スレッドの害の問題 (mc11・eil33-2・graph20-20-1rand) と、比べの neos-911970。
python sbad_t4.py → sbad_t4.jsonl (途中から再開できる)、logs_sbad/。PYTHONPATH は呼ぶ側で渡す"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / "sbad_t4.jsonl"; LOGS = D / "logs_sbad"; LOGS.mkdir(exist_ok=True)
done = {(r["problem"], r["th"], r["cfg"]) for r in map(json.loads, OUT.read_text().splitlines())} if OUT.exists() else set()
PROBS = sys.argv[1:] or ["mc11", "eil33-2", "graph20-20-1rand", "neos-911970"]
for p in PROBS:
    for th in map(int, os.environ.get("RUN_TH", "4,1").split(",")):
        for cfg in os.environ.get("RUN_CFGS", "sbad,def").split(","):
            if (p, th, cfg) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_SBADAPT_DIAG": "1"})
            if cfg.startswith("sbad"): env["ENOMOTO_MIP_SB_ADAPT"] = "1"
            if cfg.endswith("_np"): env["ENOMOTO_MIP_PARSB"] = "0"
            # 部品外し: _nocw は衝突の重みの床を 0 に、_nofb は f_B を 0 に近づける (子の伝播と床の両方が止まる)、_cwonly は SB_ADAPT なしで床 0.01 x 0.9 だけ
            if "_nocw" in cfg: env["ENOMOTO_T_MIP_SBAD_CW"] = "0"
            if "_nofb" in cfg: env["ENOMOTO_T_MIP_SBAD_BB"] = "-10"
            if "_nofa" in cfg: env["ENOMOTO_T_MIP_SBAD_AB"] = "100"
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", str(th)],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=700)
            res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            keep = [l for l in pr.stderr.splitlines() if "SBADAPT" in l or "branching:" in l or "conflict analysis" in l or "node bounds from both" in l]
            (LOGS / f"{p}_t{th}_{cfg}.log.gz").write_bytes(gzip.compress(("\n".join(keep) + "\n").encode()))
            br = [list(map(int, m.groups())) for m in (re.search(r"branching: (\d+) selections, (\d+) without.*sb LPs (\d+) iters (\d+)", l) for l in keep) if m]
            r = {"problem": p, "th": th, "cfg": cfg, "res": {k: res.get(k) for k in ("status", "objective", "bound", "nodes", "time")}, "branching": br,
                 "feat_last": [l for l in keep if "SBADAPT_FEAT" in l][-8:]}
            with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
            print(json.dumps({k: r[k] for k in ("problem", "th", "cfg", "res")} | {"sb": [b[2:] for b in br]}), flush=True)
print("SBAD_DONE", flush=True)
