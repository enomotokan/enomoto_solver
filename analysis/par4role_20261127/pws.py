"""6 節の P の 4T: 既定 (def) と ENOMOTO_MIP_PAR4ROLE=ws (ws) を、P の 58 問 (analysis/scipslow_20261119/shards_P.json) で比べる。4 スレッド・60 秒。
分担は種ごと: python pws.py <種 (1〜3)> [<値>] → pws_s<種>.jsonl (値を渡せば ENOMOTO_MIP_PAR4ROLE=<値> と既定を比べ、pws_<値>_s<種>.jsonl。6.4 節の d) (途中から再開できる。2 時間で止まったら同じコマンドで続ける)。
ビルドは d98ab271c (DUALEQ の既定化 cdb75f247 を含む) で 3 人そろえる。各行に入れたビルドの git の版 (commit) を書く。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
seed = int(sys.argv[1])
V = sys.argv[2] if len(sys.argv) > 2 else "ws"
OUT = D / (f"pws_s{seed}.jsonl" if V == "ws" else f"pws_{V}_s{seed}.jsonl")
rows = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
done = {(r["problem"], r["cfg"]) for r in rows}
TL = 60
commit = subprocess.run(["git", "rev-parse", "--short=9", "HEAD"], cwd=WT, capture_output=True, text=True).stdout.strip()
def run(p, cfg):
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1"})
    if cfg != "def": env["ENOMOTO_MIP_PAR4ROLE"] = cfg
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try: out, err = pr.communicate(timeout=TL + 600)
    except subprocess.TimeoutExpired:
        pr.kill(); out, err = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    st = [list(map(int, m)) for m in re.findall(r"PAR4ROLE: helpers a (\d+) b (\d+), helper solutions (\d+), accepted as incumbent (\d+), s heuristics while waiting (\d+) found (\d+)", err)]
    return {"problem": p, "seed": seed, "cfg": cfg, "result": res, "commit": commit,
            "w_cut": sum("PAR4ROLE w:" in l for l in err.splitlines()), "w_capped": len(re.findall(r"determ: waited [\d.]+s \(safety cap [\d.]+s, cap", err)) if cfg != "def" else None,
            "s": [sum(x[4] for x in st), sum(x[5] for x in st)] if st else None, "wall": round(time.time() - t0, 1)}
for p in P:
    for cfg in (V, "def"):
        if (p, cfg) in done: continue
        r = run(p, cfg); rows.append(r)
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        print(json.dumps({k: r[k] for k in ("problem", "seed", "cfg", "w_capped", "s")} | {"status": r["result"].get("status"), "time": r["result"].get("time")}), flush=True)
print("PWS_DONE", flush=True)
