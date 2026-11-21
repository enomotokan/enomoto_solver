"""M の行の多い問題での def と dc (`ENOMOTO_MIP_DSE_COST=1`) の比べ (README §7): python run_m.py K  (K = 0〜6、1 分担 = 1 問)
1 分担は 1 問 × {def, dc} × 種 1〜3 を 4 スレッド・600 秒で 1 本ずつ回す (約 1 時間)。
各回の stderr に経過秒を付けて logs_m/<問題>_<設定>_s<種>.log.gz、結果は res_m_<K>.jsonl に足す (途中から再開できる)。
根の潜りの秒と `MIP: DSE_COST` の数もログから読んで残す。リポジトリの今のブランチを maturin でビルドした wheel を PYTHONPATH に置いて回す。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]; L = D / "logs_m"; L.mkdir(exist_ok=True)
PROBS = ["sing44", "rail01", "n3div36", "neos-3216931-puriri", "neos-2746589-doon", "neos-3988577-wolgan", "satellites2-60-fs"]
K = int(sys.argv[1]); p = PROBS[K]
CF = {"def": {}, "dc": {"ENOMOTO_MIP_DSE_COST": "1"}}
out = D / f"res_m_{K}.jsonl"
done = {(json.loads(l)["cfg"], json.loads(l)["seed"]) for l in open(out)} if out.exists() else set()
RD = re.compile(r"MIP: root heuristic (.*diving.*?): (.*) \(([0-9.]+)s\)$")
RC = re.compile(r"MIP: DSE_COST skipped (\d+) built (\d+) built-without-estimate (\d+)")
RT = re.compile(r"TRSTAT: tree start outer (\S+) solve (\S+)")
for seed in (1, 2, 3):
    for c, extra in CF.items():
        if (c, seed) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), **extra)
        t0 = time.time()
        pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "600", "--threads", "4"],
                              cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        dives, dc, tr = [], None, None
        with gzip.open(L / f"{p}_{c}_s{seed}.log.gz", "wt") as f:
            for l in pr.stderr:
                f.write(f"[{time.time() - t0:8.2f}] {l}")
                l = l.rstrip("\n")
                if m := RD.search(l): dives.append((m.group(1), m.group(2), float(m.group(3))))
                if m := RC.search(l): dc = [int(x) for x in m.groups()]
                if tr is None and (m := RT.search(l)): tr = (float(m.group(1)), float(m.group(2)))
        so = pr.stdout.read(); pr.wait()
        try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        with open(out, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "seed": seed, "result": res, "dives": dives, "dse_cost": dc, "tree_start": tr, "wall": round(time.time() - t0, 1)}) + "\n")
        print(p, c, seed, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), dc, flush=True)
