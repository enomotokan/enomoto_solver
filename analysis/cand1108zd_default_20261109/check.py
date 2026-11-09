"""cand1108zd を既定にした後の確かめ: 環境変数なし (新しい既定)・cand1108zd を明示・precand1108zd (元に戻す形) を、1 スレッド・種 1・60 秒で比べる。
環境変数なしと明示が同じ根の下界・最初の解になれば既定が入っている。使い方: python check.py 問題,..."""
import json, os, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts")); sys.path.insert(0, str(WT / "analysis" / "halve_submip_20261030"))
import run_miplib_benchmark as rb  # noqa: E402
from run import CFGS  # noqa: E402
for p in sys.argv[1].split(","):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for name, extra in (("環境変数なし", {}), ("cand1108zd を明示", CFGS["cand1108zd"]), ("precand1108zd", CFGS["precand1108zd"])):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(extra); env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
        out = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "1"],
                             cwd=WT, env=env, capture_output=True, text=True).stdout
        r = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
        print(p, name, r.get("status"), round(r.get("time") or 0, 1), "根", r.get("root_bound"), "最初の解", r.get("first_solution_objective"), "ノード", r.get("nodes"), flush=True)
