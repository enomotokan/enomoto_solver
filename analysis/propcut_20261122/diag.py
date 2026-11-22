"""診断 ENOMOTO_MIP_DIAG_PROPCUT の 1 周: 根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1・120 秒で、打ち切り値なし (暫定解の値) と
最適値を打ち切り値に渡した回 (opt) を回し、ログの PROPCUT の行を diag.jsonl に足す。python diag.py [問題 ...]"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = sys.argv[1:] or ["swath1", "neos-911970", "misc07", "neos-1582420", "mc11", "h80x6320d", "mas76", "10teams", "neos5", "p200x1188c", "noswot", "roll3000"]
ref = {}
for l in open(WT / "scripts/miplib2017-v31.solu"):
    t = l.split()
    if len(t) >= 3 and t[0] in ("=opt=", "=best="): ref[t[1]] = float(t[2])
OUT = D / "diag.jsonl"
done = {(r["problem"], r["tag"]) for r in map(json.loads, OUT.open())} if OUT.exists() else set()
for p in PROBS:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for tag in ("inc", "opt"):
        if (p, tag) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_DIAG_PROPCUT="1", ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_NODE_LIMIT="1")
        if tag == "opt":
            o = ref[p]; env["ENOMOTO_MIP_CUTOFF"] = repr(o + 1e-6 * max(1.0, abs(o)))
        log = D / "logs" / f"{p}_{tag}.log"
        with open(log, "w") as fe:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                                cwd=WT, env=env, stdout=subprocess.PIPE, stderr=fe, text=True)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        lines = [l.strip() for l in log.read_text().splitlines() if l.startswith("PROPCUT")]
        rec = {"problem": p, "tag": tag, "opt": ref.get(p), "status": res.get("status"), "root_bound": res.get("root_bound"), "time": res.get("time"), "propcut": lines}
        with open(OUT, "a") as f: f.write(json.dumps(rec) + "\n")
        print(p, tag, res.get("status"), *lines, sep="\n  ", flush=True)
