"""片付け (2026-11-18): 既定の振る舞いが変わっていないことの確かめ。
1 スレッド・既定の種 (ENOMOTO_ の環境変数をすべて外す)・60 秒で数問を解き、(状態・目的値・ノード数) を比べる。
使い方 (リポジトリの根から、maturin develop --release で入れた後):
  python analysis/cleanup_20261118/ident.py base   … 消す前の値を base.json に書く
  python analysis/cleanup_20261118/ident.py check  … base.json と比べ、食い違いがあれば rc=1"""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
# 1 スレッドで 60 秒以内に解ける問題 (pk1・neos5・10teams・gen-ip002 は 60 秒で終わらずノード数が時間で揺れるので外した)
PROBS = ["p0201", "mas76", "rout", "noswot", "misc07", "fiber", "qnet1", "dcmulti", "blend2", "exp-1-500-5-5", "neos-860300", "khb05250"]


def run(p):
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env["ENOMOTO_MIP_THREADS"] = "1"
    r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                        "--time-limit", "60", "--threads", "1"], cwd=WT, env=env, capture_output=True, text=True, timeout=600)
    js = [l for l in r.stdout.splitlines() if l.startswith("{")]
    if not js:
        return {"status": f"crash rc={r.returncode}", "objective": None, "nodes": None, "time": None}
    d = json.loads(js[-1])
    return {"status": d["status"], "objective": d["objective"], "nodes": d["nodes"], "time": round(d["time"], 2)}


mode = sys.argv[1]
if len(sys.argv) > 2:  # 問題を選ぶ (試し用。base.json は書かない)
    PROBS = sys.argv[2].split(",")
    mode = "try"
res = {}
for p in PROBS:
    res[p] = run(p)
    print(p, res[p], flush=True)
if mode == "base":
    (D / "base.json").write_text(json.dumps(res, indent=1) + "\n")
elif mode == "check":
    base = json.loads((D / "base.json").read_text())
    bad = [p for p in PROBS if any(res[p][k] != base[p][k] for k in ("status", "objective", "nodes"))]
    print("一致" if not bad else f"食い違い: {bad}")
    sys.exit(1 if bad else 0)
