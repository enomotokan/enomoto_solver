"""最初の根の LP の値 (カット・再スタートの前) を stderr の `MIP: root LP <値>` から拾う (1 スレッド、種 1)。
診断の記録は木の探索の終わりに書くので、再スタートの前の求解の根のカットのループは記録に残らない。その穴を埋める。
使い方: python rootlp.py rootlp.jsonl"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = "neos-911970 misc07 neos5 mik-250-20-75-4 noswot".split()
M = "fiball csched008 neos-3627168-kasai neos-957323 mzzv11 tr12-30 uccase12".split()
done = {__import__("json").loads(l)["problem"] for l in open(sys.argv[1])} if Path(sys.argv[1]).exists() else set()
out = open(sys.argv[1], "a")
for p in P + M:
    if p in done:
        continue
    tl = 60  # 最初の `root LP` の行が出れば足りる
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", str(tl), "--threads", "1"],
                        cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 300)
    lp = [float(m.group(1)) for m in re.finditer(r"MIP: root LP ([-+0-9.eE]+) \(", pr.stderr)]
    restarts = len(re.findall(r"restart", pr.stderr, re.I))
    r = {"problem": p, "root_lp": lp, "restart_lines": restarts}
    out.write(json.dumps(r) + "\n"); out.flush()
    print(r, flush=True)
