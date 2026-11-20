"""gate2 候補 2 の続き: 判定が入る問題で、既定の形と含意の形のプロービングの途中の成果 (固定 + 締め付け) の伸び方を取る。
python curve.py 問題... (既定 = CLQPRB 1dgrs、種 1、4 スレッド、ENOMOTO_MIP_DEBUG_PROBE_PROGRESS)。最初の判定 (g) の 2 回のプロービングの
16 候補ごとの (候補の番号, 試した数, 固定, 締め付け) と、判定の行を curve.jsonl に足す。根の LP の大きさの行で止める。"""
import json, os, re, subprocess, sys, threading
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
for p in sys.argv[1:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_DEBUG_PROBE_PROGRESS="1")
    pr = subprocess.Popen([sys.executable, "-u", str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, errors="replace")
    killer = threading.Timer(300, pr.kill); killer.start()
    runs, cur, info, stage = [], None, [], 0
    for line in pr.stdout:
        m = re.search(r"probing progress: candidate (\d+) \(pass (\d+)\), probed (\d+), fixed (\d+), tightened (\d+)", line)
        if m:
            c, ps, pb, f, t = map(int, m.groups())
            if c == 0 and ps == 1:
                cur = []; runs.append(cur)
            if cur is not None: cur.append((c, ps, pb, f, t))
        if "CLQPRB" in line or "binaries probed" in line:
            info.append(line.strip()[:200])
        if "root LP size" in line:
            pr.kill(); break
    killer.cancel(); pr.kill(); pr.wait()
    with open(D / "curve.jsonl", "a") as f: f.write(json.dumps({"problem": p, "runs": runs[:4], "info": info[:12]}) + "\n")
    print(p, [len(r) for r in runs][:4], flush=True)
