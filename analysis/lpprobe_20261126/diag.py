"""lpprobe の診断: 前処理の LP プロービングの中身 (最初の LP・子の LP ごとの状態・反復・秒) と、根・木の時間を 1 回ずつ取る。
python diag.py OUT 問題 種 スレッド数 秒 [ENV=値 ...]  (OUT は jsonl、同じ (問題, 種, スレッド, 環境) は飛ばす)
ENOMOTO_MIP_LOG=1・ENOMOTO_MIP_LPPROBE_DIAG=1 で解き、stderr の要る行 (LPPDIAG・LP probing・probing・root LP・TRSTAT: tree start) と
その行を受け取った秒、最後の結果 (状態・目的値・ノード・時間) を残す。"""
import json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
OUT = D / sys.argv[1]
p, seed, th, tl = sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), float(sys.argv[5])
extra = dict(a.split("=", 1) for a in sys.argv[6:])
key = (p, seed, th, json.dumps(extra, sort_keys=True))
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l)
        if (r["problem"], r["seed"], r["threads"], json.dumps(r["extra"], sort_keys=True)) == key:
            sys.exit(0)
KEEP = re.compile(r"^(LPPDIAG|MIP: LP probing|MIP: probing|MIP: root LP|MIP: solving the root LP|TRSTAT: tree start|MIP: presolve|MIP: restart|MIP: symmetry: ORBPP_REPROBE|MIP:   global fixed)")
mps = rb.fetch(p, WT / ".miplib_cache")
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update({"ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(seed), "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_LPPROBE_DIAG": "1"})
env.update(extra)
t0 = time.time()
pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", str(th)],
                      cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, errors="replace")
lines = []
for l in pr.stderr:
    if KEEP.match(l):
        lines.append([round(time.time() - t0, 3), l.rstrip("\n")])
out = pr.stdout.read()
pr.wait()
try:
    res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
except Exception:  # noqa: BLE001
    res = {"status": "crash", "returncode": pr.returncode}
tree = next((t for t, l in lines if l.startswith("TRSTAT: tree start")), None)
with open(OUT, "a") as f:
    f.write(json.dumps({"problem": p, "seed": seed, "threads": th, "tl": tl, "extra": extra, "result": res, "tree_start": tree, "wall": round(time.time() - t0, 2),
                        "lines": lines, "at": time.strftime("%H:%M:%S")}) + "\n")
print(p, seed, th, extra, res.get("status"), res.get("time"), res.get("nodes"), "tree", tree, flush=True)
