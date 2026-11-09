"""GAP4 (2026-11-09): 58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で設定ごとに 1 回ずつ解き、res58.jsonl に足す (最大 RSS も)。
使い方: python bench58.py <設定名,...> <回> (種 ENOMOTO_T_MIP_SEED = 回。設定の順は回ごとに逆、ソルバーは同時に 1 つ。途中から再開できる)。
wheel は /opt/gap4whl (PYTHONPATH) を使う。"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "g4": {"ENOMOTO_MIP_GAP4": "proc"},
    "g4b": {"ENOMOTO_MIP_GAP4": "proc", "ENOMOTO_MIP_TREE_NO_PASS_BASIS": "1"},
}
OUT = D / "res58.jsonl"

def rss_mb(pid):
    try:
        for l in open(f"/proc/{pid}/status"):
            if l.startswith("VmRSS:"): return int(l.split()[1]) / 1024
    except OSError:
        pass
    return 0.0

if __name__ == "__main__":
    cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
    if rep % 2:
        cfgs = cfgs[::-1]
    done = set()
    if OUT.exists():
        for l in OUT.read_text().splitlines():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
    for p in PROBS:
        mps = rb.fetch(p, WT / ".miplib_cache")
        for c in cfgs:
            if (p, c, rep) in done:
                continue
            while (D / "PAUSE").exists():
                time.sleep(5)
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(rep), PYTHONPATH="/opt/gap4whl", **CFGS[c])
            t0 = time.time(); peak = 0.0
            pr = subprocess.Popen([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                   "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
            while pr.poll() is None:
                peak = max(peak, rss_mb(pr.pid)); time.sleep(0.5)
                if time.time() - t0 > 300:
                    pr.kill(); break
            out, _ = pr.communicate()
            try:
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "returncode": pr.returncode}
            with open(OUT, "a") as f:
                f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "max_rss_mb": round(peak), "at": time.strftime("%H:%M:%S")}) + "\n")
            print(p, c, rep, res.get("status"), res.get("time"), round(peak), flush=True)
