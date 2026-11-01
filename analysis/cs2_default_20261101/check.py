"""cs2 の既定化の確かめ (2026-11-01): 新しいビルドで、環境変数なし (new)・cs2 の環境変数を明示 (new_cs2)・元に戻す環境変数 (revert) を
同じ問題で解き、根のカットのループの行 (ラウンド数・下界・反復の上限で外した LP) と結果を並べる。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CS2 = {"ENOMOTO_T_MIP_CUT_LP_WORK": "0.1", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "5", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "flat", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "1"}
REV = {"ENOMOTO_T_MIP_CUT_LP_WORK": "0", "ENOMOTO_T_MIP_CUT_LP_WORK_PREV": "0", "ENOMOTO_MIP_CUT_LP_CAP_STOP": "0", "ENOMOTO_MIP_CUT_CAP_FROM_LOOP": "0"}
CFGS = {"new": {}, "new_cs2": CS2, "revert": REV}
for p in sys.argv[1].split(","):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CFGS.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", **e)
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        (D / "logs" / f"{p}_{c}.log").write_text(r.stderr + r.stdout)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        log = r.stderr
        m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP", log)
        rounds = len(re.findall(r"MIP: cut round \d+", log))
        print(f"{p:18s} {c:8s} {res['status']:10s} {res['time']:6.1f}s obj={res.get('objective')} root_bound={res.get('root_bound')} cuts={m.groups() if m else None} round_lines={rounds} iterlimit={log.count('IterationLimit')}", flush=True)
