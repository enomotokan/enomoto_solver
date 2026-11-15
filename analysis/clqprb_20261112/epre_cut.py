"""PRESOLVE2: ENOMOTO を根の LP を解くところまで回し (4 スレッド、種 1、ENOMOTO_MIP_LOG=1)、前処理の各段の大きさと根の LP の値・大きさを取る。
使い方: python epre.py 名前 問題 [環境変数=値 ...]   (名前は設定の略号)
出力: elog/<問題>.<名前>.log (根の LP までのログ)、epre.jsonl に 1 行 (根の LP の値・行・列・非零・その時刻)。
"MIP: root LP size" の行 (このセッションで足した記録だけの行) が出たらプロセスを止める。300 秒で出なければ止める。"""
import json, os, re, subprocess, sys, time, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
name, p = sys.argv[1], sys.argv[2]
extra = dict(kv.split("=", 1) for kv in sys.argv[3:])
(D / "elog").mkdir(exist_ok=True)
log = D / "elog" / f"{p}.{name}.cut.log"
env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", **extra)
t0 = time.time()
pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                       "--time-limit", "600", "--threads", "4"], cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
res = {"problem": p, "cfg": name, "env": extra}
killer = threading.Timer(400, pr.kill); killer.start()
with open(log, "w") as f:
    for line in pr.stdout:
        f.write(f"{time.time() - t0:8.2f} {line}")
        m = re.match(r"MIP: root LP (-?[\d.e+-]+) \((\d+) iters", line)
        if m: res["root_lp"] = float(m.group(1)); res["root_iters"] = int(m.group(2))
        m = re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", line)
        mc = re.match(r"MIP: cut loop done: .* obj (\S+) -> (\S+)", line)
        if mc: res["cut_obj"] = float(mc.group(2)); res["t_cut"] = round(time.time() - t0, 2); pr.kill(); break
        if m:
            res.update(rows=int(m.group(1)), cols=int(m.group(2)), nnz=int(m.group(3)), t=round(time.time() - t0, 2))
        if line.startswith("{"):
            res["final"] = json.loads(line)
killer.cancel(); pr.kill(); pr.wait()
with open(D / "epre_cut.jsonl", "a") as f: f.write(json.dumps(res) + "\n")
print(json.dumps(res), flush=True)
