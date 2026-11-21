"""最初の根の LP の値 (カット・再スタートの前) を stderr の `MIP: root LP <値>` から拾う (1 スレッド、種 1)。
診断の記録は木の探索の終わりに書くので、再スタートの前の求解の根のカットのループは記録に残らない。その穴を埋める。
使い方: python rootlp.py rootlp.jsonl"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
from run import P, M  # noqa: E402
out = open(sys.argv[1], "w")
for p in P + M:
    rec = [json.loads(l) for l in gzip.open(D / "logs" / f"res_{p}_t1_s1.jsonl.gz", "rt") if l.strip()]
    st = [r["start"] for r in rec if r.get("start") and r["tid"] == 0][-1]
    tl = st[3] + 30 if st[3] < 600 else 600
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", str(tl), "--threads", "1"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
    lp = [float(m.group(1)) for m in re.finditer(r"MIP: root LP ([-+0-9.eE]+) \(", pr.stderr)]
    restarts = len(re.findall(r"restart", pr.stderr, re.I))
    r = {"problem": p, "root_lp": lp, "restart_lines": restarts}
    out.write(json.dumps(r) + "\n"); out.flush()
    print(r, flush=True)
