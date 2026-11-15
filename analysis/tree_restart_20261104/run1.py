"""1 問を 1 つの設定で解き、logs/<設定>_<問題>.out (利用者向けのログと結果の行)・.err (ENOMOTO_MIP_LOG=1 の詳細) に書き、
res.jsonl に結果を足す。使い方: python analysis/tree_restart_20261104/run1.py 設定 問題 秒 [スレッド]"""
import json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "tr": {"ENOMOTO_MIP_TREE_RESTART": "1"},
    "tr2": {"ENOMOTO_MIP_TREE_RESTART": "1"},  # 2 版 (ヒューリスティクス役の起動の判定の重みと分けた後)
    "tr3": {"ENOMOTO_MIP_TREE_RESTART": "1"},  # 3 版 (全体のノード数を錠を取らずに数える)
    "trf": {"ENOMOTO_MIP_TREE_RESTART": "force"},
    "trp": {"ENOMOTO_MIP_TREE_RESTART": "1", "ENOMOTO_MIP_TREE1_POOL": "1", "ENOMOTO_MIP_RESTART_PC": "1"},
    "trpf": {"ENOMOTO_MIP_TREE_RESTART": "force", "ENOMOTO_MIP_TREE1_POOL": "1", "ENOMOTO_MIP_RESTART_PC": "1"},
}
cfg, prob, secs = sys.argv[1], sys.argv[2], float(sys.argv[3])
th = int(sys.argv[4]) if len(sys.argv) > 4 else 4
mps = rb.fetch(prob, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(CFGS[cfg]); env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_TREE_RESTART_LOG": "1", "ENOMOTO_MIP_THREADS": str(th)})
tag = f"{cfg}{'' if secs == 600 else int(secs)}_{prob}"
t0 = time.time()
with open(D / "logs" / f"{tag}.out", "w") as fo, open(D / "logs" / f"{tag}.err", "w") as fe:
    subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(secs), "--threads", str(th)],
                   stdout=fo, stderr=fe, env=env, timeout=secs * 1.5 + 120)
r = {}
for line in reversed((D / "logs" / f"{tag}.out").read_text().splitlines()):
    if line.startswith("{"):
        r = json.loads(line); break
err = (D / "logs" / f"{tag}.err").read_text(errors="replace")
r["restarts_tree"] = [l for l in err.splitlines() if "MIP: TREE restart:" in l]
r["restart_lines"] = [l for l in (D / "logs" / f"{tag}.out").read_text().splitlines() if l.startswith("Restart")]
r["warnings"] = err.count("MIP: WARNING")
with open(D / "res.jsonl", "a") as f:
    f.write(json.dumps({"problem": prob, "cfg": cfg, "secs": secs, "threads": th, "result": r, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
print(prob, cfg, r.get("status"), r.get("time"), r.get("bound"), r.get("objective"), len(r["restarts_tree"]), flush=True)
