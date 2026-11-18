"""1 問を設定ごとに回す (lpinf の数え上げつき): python one.py 問題 TAG 種,種 秒 [KEY=VAL ...]。res_one.jsonl に足す。
ログ (ENOMOTO_MIP_LOG=1) は一時ファイルに出し、統計の行 (際どい実行不能・ノード数など) だけ拾う。ソルバーは同時に 1 つ、4 スレッド。"""
import json, os, re, subprocess, sys, tempfile, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
prob, tag, seeds, tl = sys.argv[1], sys.argv[2], [int(s) for s in sys.argv[3].split(",")], float(sys.argv[4])
extra = dict(a.split("=", 1) for a in sys.argv[5:])
mps = rb.fetch(prob, WT / ".miplib_cache")
for s in seeds:
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(extra); env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(s), "ENOMOTO_MIP_LOG": "1"})
    lf = Path(tempfile.gettempdir()) / f"one_{os.getpid()}.log"
    t0 = time.time()
    with open(lf, "w") as err:
        p = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "4"],
                           cwd=WT, env=env, stdout=subprocess.PIPE, stderr=err, text=True)
    try: res = json.loads([l for l in p.stdout.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "rc": p.returncode}  # noqa: BLE001
    li = None; keep = []
    for l in lf.read_text(errors="replace").splitlines():
        if "marginal LP infeasibility" in l:
            li = [int(v) for v in re.findall(r"\d+", l.split("cumulative):")[1])]
        if "LP infeasibility certificates" in l:
            keep.append("hist " + l.split(":")[-1].strip())
        if l.startswith("Result:") or "Root LP:" in l:
            keep.append(l[:200])
    lf.unlink()
    row = {"problem": prob, "tag": tag, "seed": s, "tl": tl, "env": extra, "result": res, "lpinf": li, "log": keep[-4:], "wall": round(time.time() - t0, 1)}
    with open(D / "res_one.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
    print(prob, tag, s, res.get("status"), res.get("objective"), res.get("bound"), res.get("nodes"), res.get("time"), li, flush=True)
