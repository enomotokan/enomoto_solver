"""4 スレッドの根 (ENOMOTO_MIP_NODE_LIMIT=1) で、主の根の LP の同時実行の勝ち手 (ENOMOTO_MIP_XOLAZY_LOG の winner) を見る。
ipmxm は内点法 + クロスオーバーが根の LP を解く場面でだけ働くので、内点法が勝たない問題では 4 スレッドでも計算を変えない。
python race.py 問題... > race.txt (既定の設定で、種 1〜3)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[2]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
for p in sys.argv[1:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    out = []
    for s in (1, 2, 3):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_XOLAZY_LOG": "1", "ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": str(s), "ENOMOTO_MIP_NODE_LIMIT": "1"})
        err = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "600", "--threads", "4"],
                             cwd=WT, env=env, capture_output=True, text=True, timeout=1200).stderr
        w = [json.loads(m).get("winner") for m in re.findall(r"XOLAZY (\{.*\"ev\":\"rootlp\".*\})", err)]
        out.append(f"種{s}: {w}")
    print(p, "|", " / ".join(out), flush=True)
