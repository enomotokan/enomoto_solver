"""前処理の強制行の修正 (2026-11-01) の確かめ: 問題を設定ごとに 1 本ずつ解き、結果を results.jsonl に足す。
4 スレッド・60 秒、ソルバーは同時に 1 つだけ。使い方: python run.py 設定,... 問題,...|58 回数 [タグ]。
設定は CFGS (def = 環境変数なし、ns250s・n50・t10 は試作)。ログは logs/<問題>_<設定>_<回>.log.gz。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
ON = {"ENOMOTO_MIP_SUBMIP_NOSOL_STOP": "1"}
CFGS = {
    "def": {},
    # submip_nodes_20261101 の ns250s (h80x6320d で 2 回誤った最適値を返した形)
    "ns250s": {"ENOMOTO_T_MIP_SUBMIP_NOSOL_CAP": "250", "ENOMOTO_MIP_SUBMIP_NOSOL_CAP_SERIAL": "1", "ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    # submip_nosol_stop_20261101 の n50・t10
    "n50": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="50"),
    "t10": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="100", ENOMOTO_T_MIP_SUBMIP_NOSOL_TIME_FRAC="0.1"),
    # 修正前の判定 (余裕が PROPAGATE_EPS 以下なら強制行)
    "old": {"ENOMOTO_T_FORCING_SLACK_REL": "0"},
}
if __name__ == "__main__":
    cfgs = sys.argv[1].split(",")
    if sys.argv[2] == "58":
        from probs import PROBS
        probs = PROBS
    else:
        probs = sys.argv[2].split(",")
    reps = int(sys.argv[3])
    tag = sys.argv[4] if len(sys.argv) > 4 else ""
    base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    (D / "logs").mkdir(exist_ok=True)
    for r in range(1, reps + 1):
        for p in probs:
            for c in cfgs:
                out = D / "logs" / f"{p}_{c}{tag}_{r}.log.gz"
                if out.exists():
                    continue
                env = dict(base, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                t0 = time.time()
                pr = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"),
                                     "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                wall = time.time() - t0
                with gzip.open(out, "wt") as f:
                    f.write(f"# wall={wall:.1f} cfg={c} env={json.dumps(CFGS[c])}\n" + pr.stderr + "\n" + pr.stdout)
                res = None
                for line in pr.stdout.splitlines():
                    if line.startswith("{") and '"status"' in line:
                        res = json.loads(line)
                with open(D / "results.jsonl", "a") as f:
                    f.write(json.dumps({"prob": p, "cfg": c + tag, "rep": r, "wall": round(wall, 1), "res": res}) + "\n")
                print(p, c + tag, r, f"{wall:.1f}s", (res or {}).get("status"), (res or {}).get("objective"), (res or {}).get("time"), flush=True)
