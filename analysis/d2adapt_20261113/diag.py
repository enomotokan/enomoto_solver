"""D2 適応版の下調べ (スレッドは環境変数 TH、既定 4): 1 問を cand1114 (+ 設定 cfg.py) で解き、ノードの最初の LP の反復を潜りの子 / 待ち行列から取ったノードで比べる
(ENOMOTO_PROF_TREE_PAR の TREEPAR 行をスレッドで足す)。基底の復元の不一致と D2 の計数も拾う。
python diag.py 設定 秒 種 問題... [-- 名前=値 ...]   結果は diag.jsonl に 1 行 (ログは logs/<問題>_<設定>_s<種>.log.gz、コミットしない)"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
sys.path.insert(0, str(D))
import run_miplib_benchmark as rb  # noqa: E402
from cfg import CFGS  # noqa: E402
cfg, tl, seed = sys.argv[1], sys.argv[2], sys.argv[3]
rest = sys.argv[4:]
probs, extra = (rest[:rest.index("--")], rest[rest.index("--") + 1:]) if "--" in rest else (rest, [])
for p in probs:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(CFGS[cfg], ENOMOTO_MIP_THREADS=os.environ.get("TH", "4"), ENOMOTO_T_MIP_SEED=seed, ENOMOTO_MIP_LOG="1", ENOMOTO_PROF_TREE_PAR="1")
    env.update(dict(kv.split("=", 1) for kv in extra))
    t0 = time.time()
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", os.environ.get("TH", "4")],
                       cwd=WT, env=env, capture_output=True, text=True)
    (D / "logs").mkdir(exist_ok=True)
    tag = cfg + ("_t" + os.environ.get("TH", "4")) + ("_" + "_".join(extra) if extra else "")
    with gzip.open(D / "logs" / f"{p}_{tag}_s{seed}.log.gz", "wt") as f:
        f.write(r.stderr)
    txt = r.stderr
    try:
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "rc": r.returncode}
    kn = [0, 0, 0]; ki = [0.0, 0.0, 0.0]
    for m in re.finditer(r"first LP \[dive (\d+) ([\d.]+)it [\d.]+ms \| queue (\d+) ([\d.]+)it [\d.]+ms \| pool (\d+) ([\d.]+)it", txt):
        g = m.groups()
        for k in range(3):
            kn[k] += int(g[2 * k]); ki[k] += int(g[2 * k]) * float(g[2 * k + 1])
    rs = re.findall(r"basis restores (\d+) \(basic count mismatch (\d+)\)", txt)
    d2 = re.findall(r"MIP: HL2_D2: .*", txt)
    ad = re.findall(r"MIP: D2B?ADAPT.*", txt)
    row = {"problem": p, "cfg": cfg, "seed": int(seed), "tl": float(tl), "extra": extra, "th": os.environ.get("TH", "4"), "status": res.get("status"), "time": res.get("time"),
           "nodes": res.get("nodes"), "obj": res.get("objective"), "dive": [kn[0], round(ki[0] / max(kn[0], 1), 1)],
           "queue": [kn[1], round(ki[1] / max(kn[1], 1), 1)], "pool": [kn[2], round(ki[2] / max(kn[2], 1), 1)],
           "restores": [[int(a), int(b)] for a, b in rs], "d2": d2[-1:] , "adapt": ad[-4:], "wall": round(time.time() - t0, 1)}
    with open(D / "diag.jsonl", "a") as f:
        f.write(json.dumps(row) + "\n")
    print(json.dumps(row), flush=True)
