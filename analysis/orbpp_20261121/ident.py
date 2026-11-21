"""既定化の確かめ: 1 スレッドで misc07・neos-911970・brazil3 を回し、ノード・目的値・「ORBPP cond:」の行を出す。
python ident.py TAG [ENOMOTO_X=V ...] >> ident.txt (変更前のビルドで ENOMOTO_MIP_ORBPP=cond、変更後のビルドで何も付けずに回して比べる)"""
import os, re, subprocess, sys, json
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
tag = sys.argv[1]
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
for a in sys.argv[2:]:
    k, v = a.split("=", 1); env[k] = v
for p, tl in (("misc07", 120), ("neos-911970", 300), ("brazil3", 120)):
    pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", str(tl), "--threads", "1"],
                        cwd=WT, env=env, capture_output=True, text=True)
    res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
    m = re.findall(r"ORBPP cond: .*?-> (static|reverted to dynamic)", pr.stderr)
    fx = re.findall(r"ORBPP cond: .*new (\d+)", pr.stderr)
    print(tag, p, res["status"], res["nodes"], res.get("objective"), res.get("bound"), m, fx, flush=True)
