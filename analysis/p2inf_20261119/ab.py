"""P2-INF: 1 問を {v019, def, def から既定化した機能を 1 つずつ切った形} × 種で回し、res_ab_<問題>.jsonl に足す (途中から再開できる)。
python ab.py 問題 秒 スレッド 種,種,... [設定,...]。v019 は <repo>/.v019env の python (analysis/final_20261118/setup_v019.sh で作る)。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
# 0.1.9 の後に既定化した機能を 1 つずつ切る (docs/improvement_history.md の 2026-11-17〜18 の採用の節)
OFF = {
    "def": {},
    "v019": {},
    "no_can": {"ENOMOTO_IPM_G5CANCEL": "0"},
    "no_clqprb": {"ENOMOTO_MIP_CLQPRB": "0"},
    "no_seq": {"ENOMOTO_MIP_G5SEQ": "0"},
    "no_rfs30": {"ENOMOTO_T_MIP_ROBUST_FJ_FEAS_SECS": "inf"},
    "no_drevmf": {"ENOMOTO_MIP_DOMCOL_REV": "0", "ENOMOTO_T_MIP_DOMCOL_MIN_FRAC": "0"},
    "no_sw2": {"ENOMOTO_T_MIP_SUBWAIT": "0"},
    "no_spl": {"ENOMOTO_MIP_SPARSIFY_LATE": "0"},
    "no_s3g": {"ENOMOTO_MIP_S3GAP": "0"},
    "no_rin": {"ENOMOTO_MIP_RACE_ITERCAP_NOWAIT": "0"},
    "no_vubagg": {"ENOMOTO_MIP_VUBAGG": "0"},
    "no_orbpp": {"ENOMOTO_MIP_ORBPP": "0"},
}
p, tl, th = sys.argv[1], float(sys.argv[2]), sys.argv[3]
seeds = [int(s) for s in sys.argv[4].split(",")]
cfgs = sys.argv[5].split(",") if len(sys.argv) > 5 else list(OFF)
mps = rb.fetch(p, WT / ".miplib_cache")
out = D / f"res_ab_{p}.jsonl"
done = {(r["cfg"], r["seed"], r["th"]) for r in map(json.loads, out.read_text().splitlines())} if out.exists() else set()
for s in seeds:
    for c in cfgs:
        if (c, s, th) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(OFF[c]); env["ENOMOTO_MIP_THREADS"] = th; env["ENOMOTO_T_MIP_SEED"] = str(s)
        py = str(WT / ".v019env/bin/python") if c == "v019" else sys.executable
        t0 = time.time()
        r = subprocess.run([py, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", th],
                           cwd=WT, env=env, capture_output=True, text=True)
        try: res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": r.returncode}  # noqa: BLE001
        row = {"problem": p, "cfg": c, "seed": s, "th": th, "tl": tl, "result": res, "wall": round(time.time() - t0, 1)}
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(c, s, th, res.get("status"), round(res.get("time") or 0, 1), res.get("nodes"), flush=True)
