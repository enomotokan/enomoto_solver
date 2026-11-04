"""58 問 (analysis/cbcimpl_20261029/probs.py) を 4 スレッド・60 秒で、今の既定 (def) と試作 (memb: ENOMOTO_MIP_MEMB=rcq) で 1 回ずつ解き、
bench58.jsonl に足す (2026-11-04、memcap_b)。問題ごとに設定の順を入れ替える。最大 RSS は wait4 の ru_maxrss。
使い方: python analysis/memcap_b_20261104/bench58.py [回]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
CFGS = {"def": {}, "memb": {"ENOMOTO_MIP_MEMB": "rcq"}}
OUT = D / "bench58.jsonl"
rep = int(sys.argv[1]) if len(sys.argv) > 1 else 1
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
for i, p in enumerate(PROBS):
    cfgs = list(CFGS) if (i + rep) % 2 == 0 else list(CFGS)[::-1]
    for c in cfgs:
        if (p, c, rep) in done:
            continue
        env = dict(os.environ, ENOMOTO_MIP_THREADS="4", **CFGS[c])
        cmd = [sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", "60", "--threads", "4"]
        t0 = time.time(); tmp = Path(f"/tmp/b58_{os.getpid()}.out")
        with open(tmp, "w") as fo:
            pr = subprocess.Popen(cmd, cwd=WT, env=env, stdout=fo, stderr=subprocess.DEVNULL)
            while True:
                wp, st, ru = os.wait4(pr.pid, os.WNOHANG)
                if wp: break
                if time.time() - t0 > 300: pr.kill()
                time.sleep(0.2)
        out = tmp.read_text()
        try:
            res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        except Exception as e:  # noqa: BLE001
            res = {"status": "crash", "error": str(e)[:300], "rc": os.waitstatus_to_exitcode(st)}
        res["maxrss_mb"] = round(ru.ru_maxrss / 1024, 1)
        with open(OUT, "a") as f:
            f.write(json.dumps({"problem": p, "cfg": c, "rep": rep, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
        print(p, c, rep, res.get("status"), res.get("time"), flush=True)
