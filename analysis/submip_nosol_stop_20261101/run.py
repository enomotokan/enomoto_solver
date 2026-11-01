"""解なしのサブ MIP の打ち切り (ENOMOTO_MIP_SUBMIP_NOSOL_STOP) の試作の計測 (2026-11-01)。
4 スレッド・60 秒、ソルバーは同時に 1 つだけ。ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1 (SMT の行で解なしのサブ MIP を数える)。
回 → 問題 → 設定の順に回す (時間とともに機械の速さが変わっても設定の間で偏らないように)。
使い方: python run.py [回数] [問題,...] [設定,...]。ログは logs/<設定>/<問題>_r<回>.log.gz、結果の行は results.jsonl。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
PROBS = "n5-3 neos-1582420 neos-2978193-inde h80x6320d qiu mc11 physiciansched6-2 10teams binkar10_1 blp-ar98 mas76".split()
ON = {"ENOMOTO_MIP_SUBMIP_NOSOL_STOP": "1"}
CFGS = {
    "def": {},
    "n200": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="200"),
    "n100": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="100"),
    "n50": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="50"),
    # 2 回目のビルド (別のセッションの RENS の試作のコミットを取り込んだ木。既定の振る舞いは変わらないはずだが、同じビルドの既定 def2 も回す)
    "def2": {},
    # (ノード数と時間の両方で止める形): 100 ノード以上かつサブ MIP の時間の上限の 10%・20% 以上
    "t10": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="100", ENOMOTO_T_MIP_SUBMIP_NOSOL_TIME_FRAC="0.1"),
    "t20": dict(ON, ENOMOTO_T_MIP_SUBMIP_NOSOL_NODES="100", ENOMOTO_T_MIP_SUBMIP_NOSOL_TIME_FRAC="0.2"),
}

if __name__ == "__main__":
    reps = int(sys.argv[1]) if len(sys.argv) > 1 else 3
    probs = sys.argv[2].split(",") if len(sys.argv) > 2 and sys.argv[2] else PROBS
    cfgs = sys.argv[3].split(",") if len(sys.argv) > 3 else ["def", "n200", "n100", "n50"]
    base = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    for r in range(1, reps + 1):
        for p in probs:
            for c in cfgs:
                L = D / "logs" / c
                L.mkdir(parents=True, exist_ok=True)
                out = L / f"{p}_r{r}.log.gz"
                if out.exists():
                    continue
                env = dict(base, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SUBMIP_VERBOSE="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                t0 = time.time()
                pr = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                     "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                wall = time.time() - t0
                with gzip.open(out, "wt") as f:
                    f.write(f"# wall={wall:.1f} cfg={c} env={json.dumps(CFGS[c])}\n" + pr.stderr + "\n" + pr.stdout)
                res = None
                for line in pr.stdout.splitlines():
                    if line.startswith("{") and '"status"' in line:
                        res = json.loads(line)
                with open(D / "results.jsonl", "a") as f:
                    f.write(json.dumps({"prob": p, "cfg": c, "rep": r, "wall": round(wall, 1), "res": res}) + "\n")
                print(p, c, r, f"{wall:.1f}s", (res or {}).get("status"), (res or {}).get("objective"), (res or {}).get("time"), flush=True)
