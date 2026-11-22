"""gainstall の比べ (analysis/gainstall_20261122): python run_root.py [MODE...] (MODE は root・full、既定は両方)。
1 スレッド・種 1 で、問題 PROBS × 設定 CFGS を回し、res_root.jsonl・res_full.jsonl に 1 行ずつ足す (途中から再開できる)。
- root: 根だけ (`ENOMOTO_MIP_NODE_LIMIT=1`、120 秒)。外側の最初の根のカットのループの `CUTSTOP` の行 (止めた理由・ラウンド数・始めと終わりの LP の値) と根の下界。
- full: 120 秒の通しの結果 (状態・時間・ノード・根の下界)。
ログは logs/<問題>_<設定>_<MODE>.log.gz (CUTSTOP_LOG の `STALLGAIN` の行を含む)。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "g1e-3": {"ENOMOTO_MIP_STALL_GAIN": "1e-3"}, "g1e-2": {"ENOMOTO_MIP_STALL_GAIN": "1e-2"}, "g3e-2": {"ENOMOTO_MIP_STALL_GAIN": "3e-2"},
        "w1e-2": {"ENOMOTO_MIP_STALL_GAIN": "1e-2w"}, "w3e-2": {"ENOMOTO_MIP_STALL_GAIN": "3e-2w"}}
# 2 回目 (窓つき、3 節): `GS_RUN=2` で設定を def・g3e-2・w1e-2・w3e-2 に、問題に neos-827175・graph20-20-1rand を足し、res_<MODE>2.jsonl に
RUN2 = os.environ.get("GS_RUN") == "2"
if RUN2: CFGS = {k: CFGS[k] for k in ("def", "g3e-2", "w1e-2", "w3e-2")}
# cutstop・fracstall の 10 問と、根の下界の絶対値が 1 未満の問題 (P・M の def の root_bound から)
PROBS = ["swath1", "beasleyC3", "mik-250-20-75-4", "timtab1", "mc11", "roll3000", "neos17", "neos-911970", "neos-1582420", "h80x6320d",
         "neos-5188808-nattai", "neos-5195221-niemur", "markshare_4_0", "pk1", "mad", "ns1208400"]
if RUN2: PROBS += ["neos-827175", "graph20-20-1rand"]
TL = 120
modes = sys.argv[1:] or ["root", "full"]
(D / "logs").mkdir(exist_ok=True)
for mode in modes:
    out = D / f"res_{mode}{'2' if RUN2 else ''}.jsonl"
    done = set()
    if out.exists():
        for l in out.read_text().splitlines():
            if l.strip():
                r = json.loads(l); done.add((r["problem"], r["cfg"]))
    for p in PROBS:
        for cfg in CFGS:
            if (p, cfg) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(CFGS[cfg]); env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1"})
            if mode == "root": env["ENOMOTO_MIP_NODE_LIMIT"] = "1"
            t0 = time.time()
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "1"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, timeout=TL + 300)
            log = pr.stderr
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
            with gzip.open(D / "logs" / f"{p}_{cfg}_{mode}{'2' if RUN2 else ''}.log.gz", "wt") as f: f.write(log + "\n" + pr.stdout)
            cs = [json.loads(m) for m in re.findall(r"^CUTSTOP (\{.*\})$", log, re.M)]
            first = next((c for c in cs if c.get("restarts") == 0 and not c.get("bg")), None)
            row = {"problem": p, "cfg": cfg, "mode": mode, "result": res, "wall": round(time.time() - t0, 1), "cutstop_first": first, "cutstop_all": cs,
                   "n_stallgain": len(re.findall(r"STALLGAIN round", log))}
            with open(out, "a") as f: f.write(json.dumps(row) + "\n")
            print(mode, p, cfg, res.get("status"), res.get("time"), res.get("root_bound"), first and (first["reason"], first["rounds"]), flush=True)
