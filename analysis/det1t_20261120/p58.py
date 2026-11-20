"""P の 58 問を 1 スレッド・種 1 で 2 回ずつ (順に) 解き、ノードの最初の LP の記録 (MEMBQ_DIAG・MEMBQ_PRE の外側の求解の行) と結果が
一致するかを p58_<名前>.jsonl に書く。使い方: python p58.py 名前 秒 [環境変数=値 ...]。途中から続きを回せる。
一致の判定: 記録の共通の長さまで (最後の 1 行は時間切れで切れうるので除く) 同じで、解けた回は状態・目的値・ノード数も同じ。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
import run_miplib_benchmark as rb  # noqa: E402
from probs import PROBS  # noqa: E402
name, tl = sys.argv[1], sys.argv[2]
extra = dict(a.split("=", 1) for a in sys.argv[3:])
OUT = D / f"p58_{name}.jsonl"
done = {json.loads(l)["problem"] for l in open(OUT)} if OUT.exists() else set()
def run(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_MEMBQ_DIAG="1", ENOMOTO_MIP_THREADS="1", ENOMOTO_T_MIP_SEED="1", ENOMOTO_MIP_LOG="1")
    env.update(extra)
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(rb.fetch(p, WT / ".miplib_cache")), "--time-limit", tl, "--threads", "1"],
                       cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 4 + 300)
    seq = [re.sub(r"t [0-9.]+ ", "", l) for l in r.stderr.splitlines() if (l.startswith("MEMBQ_DIAG") or l.startswith("MEMBQ_PRE")) and " sub 0 " in l]
    res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
    return seq, res
for p in PROBS:
    if p in done:
        continue
    (sa, ra), (sb, rb_) = run(p), run(p)
    n = min(len(sa), len(sb))
    first = next((i for i in range(n) if sa[i] != sb[i]), None)
    solved = ra["status"] == "optimal" and rb_["status"] == "optimal"
    same_res = (ra["status"], ra.get("objective"), ra["nodes"]) == (rb_["status"], rb_.get("objective"), rb_["nodes"])
    same = (first is None or first >= n - 1) and (same_res if solved else True)
    rec = {"problem": p, "cfg": name, "same": same, "common": n, "first_diff": first, "status": [ra["status"], rb_["status"]], "nodes": [ra["nodes"], rb_["nodes"]],
           "time": [round(ra["time"], 2), round(rb_["time"], 2)]}
    with open(OUT, "a") as f:
        f.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
