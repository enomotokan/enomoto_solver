"""1 スレッド・同じ種で同じ問題を 2 回 (順に) 解き、ノードの最初の LP の記録 (ENOMOTO_MIP_MEMBQ_DIAG の外側の求解の行) を並べて、最初に食い違う
ノードを出す。使い方: python pair.py 問題 秒 回数 [環境変数=値 ...] → pair.jsonl に 1 組 1 行。時間切れで片方が先に止まった分は数えない (共通の長さまで比べる)。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
p, tl, reps = sys.argv[1], sys.argv[2], int(sys.argv[3])
extra = dict(a.split("=", 1) for a in sys.argv[4:])
mps = rb.fetch(p, WT / ".miplib_cache")
def start():
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_MEMBQ_DIAG="1", ENOMOTO_MIP_DET_DIAG="1", ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    env.update(extra)
    return subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", "1"],
                            cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
for r in range(reps):
    # 2 回は順に走らせる (同時だと片方が CPU を取られて大きく遅れ、時間切れの位置が離れる)
    outs = []
    for _ in range(2):
        outs.append(start().communicate())
    seqs, res = [], []
    for o, e in outs:
        seqs.append([re.sub(r"t [0-9.]+ ", "", l) for l in e.splitlines() if ((l.startswith("MEMBQ_DIAG") or l.startswith("MEMBQ_PRE")) and " sub 0 " in l) or l.startswith("DET_DIAG")])
        res.append(json.loads([l for l in o.splitlines() if l.startswith("{")][-1]))
    n = min(len(s) for s in seqs)
    first = next((i for i in range(n) if seqs[0][i] != seqs[1][i]), None)
    if first is not None:
        (D / "logs").mkdir(exist_ok=True)
        for k, (o, e) in enumerate(outs):
            (D / "logs" / f"{p}_r{r}_{k}.log").write_text(e)
        print("A:", seqs[0][max(0, first - 2):first + 2], "\nB:", seqs[1][max(0, first - 2):first + 2], flush=True)
    rec = {"problem": p, "extra": extra, "rep": r, "common": n, "first_diff": first, "nodes": [x["nodes"] for x in res], "status": [x["status"] for x in res],
           "time": [round(x["time"], 1) for x in res]}
    with open(D / "pair.jsonl", "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
