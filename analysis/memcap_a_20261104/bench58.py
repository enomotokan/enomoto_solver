"""memcap_a (2026-11-04): 58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で設定 (run_long.py の CFGS) ごとに解き、
bench58.jsonl に足す。最大 RSS (1 秒ごとの VmRSS) と `MIP: MEMCAP:` の行も残す。問題ごとに設定の順を入れ替える。
使い方: python analysis/memcap_a_20261104/bench58.py def,memcap 回"""
import json, os, subprocess, sys, time, tempfile
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {"def": {}, "memcap": {"ENOMOTO_MIP_MEMCAP": "1"}}
OUT = D / "bench58.jsonl"
cfgs, rep = sys.argv[1].split(","), int(sys.argv[2])
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for i, p in enumerate(PROBS):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c in (cfgs if (i + rep) % 2 == 0 else cfgs[::-1]):
        if (p, c, rep) in done:
            continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", **CFGS[c])
        t0 = time.time(); peak = 0.0
        with tempfile.TemporaryFile("w+") as ef:
            pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                                  cwd=WT, env=env, stdout=subprocess.PIPE, stderr=ef, text=True)
            while pr.poll() is None:
                try:
                    peak = max(peak, next(int(l.split()[1]) / 1024 for l in open(f"/proc/{pr.pid}/status") if l.startswith("VmRSS:")))
                except (OSError, StopIteration):
                    pass
                time.sleep(1)
                if time.time() - t0 > 300: pr.kill(); break
            out, _ = pr.communicate()
            ef.seek(0); err = ef.read()
        try:
            res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception:  # noqa: BLE001
            res = {"status": "crash", "returncode": pr.returncode, "error": err[-300:]}
        mc = [l for l in err.splitlines() if "MEMCAP" in l]
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "max_rss_mb": round(peak), "memcap": mc, "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), round(peak), flush=True)
