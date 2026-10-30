"""根のカットの像をプールに入れる試作 (`ENOMOTO_MIP_SYM_CUTS`) を、既定と比べる (2026-10-30、4 スレッド・60 秒)。
使い方: python run.py <設定名,...> <問題,...> <回数>。ログは <問題>_<設定>_<回>.log。"""
import os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
C = WT / ".miplib_cache"
CFGS = {"def": {}, "sym": {"ENOMOTO_MIP_SYM_CUTS": "1"}}
if __name__ == "__main__":
    cfgs, probs, reps = sys.argv[1].split(","), sys.argv[2].split(","), int(sys.argv[3])
    for rep in range(reps):
        for p in probs:
            for c in cfgs:
                out = D / f"{p}_{c}_{rep}.log"
                if out.exists():
                    continue
                env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", **CFGS[c])
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(C / f"{p}.mps"),
                                    "--time-limit", "60", "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=400)
                out.write_text(r.stderr + "\n" + r.stdout)
                print(out.name, r.stdout.strip().splitlines()[-1][:170] if r.stdout.strip() else "NO OUTPUT", flush=True)
