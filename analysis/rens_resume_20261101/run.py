"""RENS のやり直しを続きから継続する試作 (ENOMOTO_MIP_RENS_RESUME) の計測 (2026-11-01)。4 スレッド・60 秒、ソルバーは同時に 1 つだけ。
ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1 (サブ MIP の中の経過と `SMT ...` の行も出す)。
使い方: python run.py 設定[,設定...] 問題[,問題...] 回数 [ログのディレクトリ]
設定: def (既定)・real (既定 + ENOMOTO_MIP_RENS_MORE_NODES_REAL=1)・resume (既定 + ENOMOTO_MIP_RENS_RESUME=1)。
ログは logs/<設定>/<問題>_r<回>.log (あれば飛ばす)。回ごとに設定を入れ替えて回す (時間の偏りを散らす)。"""
import os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb

CONF = {
    "def": {},
    "real": {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1"},
    "resume": {"ENOMOTO_MIP_RENS_RESUME": "1"},
}

if __name__ == "__main__":
    confs = sys.argv[1].split(",")
    probs = sys.argv[2].split(",")
    reps = int(sys.argv[3])
    L = D / (sys.argv[4] if len(sys.argv) > 4 else "logs")
    for p in probs:
        rb.fetch(p, C)
    for k in range(1, reps + 1):
        for p in probs:
            for c in confs:
                out = L / c / f"{p}_r{k}.log"
                if out.exists() or Path(str(out) + ".gz").exists():
                    continue
                out.parent.mkdir(parents=True, exist_ok=True)
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SUBMIP_VERBOSE="1", ENOMOTO_MIP_THREADS="4", **CONF[c])
                t0 = time.time()
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(f"# wall={time.time()-t0:.1f}\n" + r.stderr + "\n" + r.stdout)
                print(c, out.name, f"{time.time()-t0:.1f}s", r.stdout.strip().splitlines()[-1][:160] if r.stdout.strip() else "NO OUTPUT", flush=True)
