"""ビルドの版の比べ (analysis/buildopt_20261122): python run_var.py TAG 版1 版2 ... (版は var/<版>、build_var.sh で作る)。
1 スレッド・種 1・時間上限 TL 秒で、問題 PROBS × 版 × 回 REPS を、問題ごとに版の順を回しながら解き、res_<TAG>.jsonl に足す (途中から再開できる)。
版を読ませるには PYTHONPATH=var/<版> を前に置く (編集可能な入れ方の python/ は .pth で後ろに来る)。行に読んだ .so の sha1 も書く。
環境変数: PROBS (カンマ区切り、既定は下の 15 問)、TL (既定 200)、REPS (既定 2)、TH (スレッド、既定 1)。"""
import hashlib, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
# 根の LP が重い (ノード 0): air03・neos8・nw04・physiciansched6-2・neos-860300。分離の重い根 (ノード少): h80x6320d・exp-1-500-5-5・qnet1。
# ノードの多い木 (小さい LP): markshare_4_0・mas76。中くらいの木: mik-250-20-75-4・rout・noswot・neos-911970・qiu。
PROBS = ["air03", "neos8", "nw04", "physiciansched6-2", "neos-860300", "h80x6320d", "exp-1-500-5-5", "qnet1",
         "markshare_4_0", "mas76", "mik-250-20-75-4", "rout", "noswot", "neos-911970", "qiu"]
if os.environ.get("PROBS"): PROBS = os.environ["PROBS"].split(",")
TL = int(os.environ.get("TL", "200")); REPS = int(os.environ.get("REPS", "2")); TH = os.environ.get("TH", "1")
TAG, VARS = sys.argv[1], sys.argv[2:]
SHA = {}
for v in VARS:
    so = next((D / "var" / v / "enomoto_solver").glob("_core*.so"))
    SHA[v] = hashlib.sha1(so.read_bytes()).hexdigest()[:8]
    chk = subprocess.run([sys.executable, "-c", "import enomoto_solver._core as c; print(c.__file__)"], env={**os.environ, "PYTHONPATH": str(D / "var" / v)},
                         stdout=subprocess.PIPE, text=True, cwd=WT / "scripts").stdout.strip()
    assert chk.startswith(str(D / "var" / v)), (v, chk)
out = D / f"res_{TAG}.jsonl"
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["var"], r["rep"]))
for rep in range(REPS):
    for i, p in enumerate(PROBS):
        order = VARS[(i + rep) % len(VARS):] + VARS[:(i + rep) % len(VARS)]
        for v in order:
            if (p, v, rep) in done: continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            env = {k: val for k, val in os.environ.items() if not k.startswith("ENOMOTO_") and k != "PYTHONPATH"}
            env.update({"PYTHONPATH": str(D / "var" / v), "ENOMOTO_MIP_THREADS": TH, "ENOMOTO_T_MIP_SEED": "1"})
            t0 = time.time()
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", TH],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True, timeout=TL + 300)
            try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
            except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
            row = {"problem": p, "var": v, "rep": rep, "so_sha1": SHA[v], "th": TH, "tl": TL, "result": res, "wall": round(time.time() - t0, 2)}
            with open(out, "a") as f: f.write(json.dumps(row) + "\n")
            print(rep, p, v, res.get("status"), res.get("time"), res.get("nodes"), res.get("objective"), flush=True)
