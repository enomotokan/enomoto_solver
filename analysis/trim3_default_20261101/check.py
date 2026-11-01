"""trim3 の既定化の確かめ (2026-11-01): 新しいビルドで、環境変数なし (new)・trim3 の環境変数を明示 (new_t3)・元に戻す環境変数 (revert) を
同じ問題で解き、ログの行 (`RENS (more nodes)`・`skipped`・深さ 3 の RENS) と結果を並べる。4 スレッド・60 秒、ENOMOTO_MIP_LOG=1 ENOMOTO_MIP_SUBMIP_VERBOSE=1。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
T3 = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "1", "ENOMOTO_MIP_RENS_RESUME": "1", "ENOMOTO_MIP_RENS_MAX_DEPTH": "2", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1"}
REV = {"ENOMOTO_MIP_RENS_MORE_NODES_REAL": "0", "ENOMOTO_MIP_RENS_RESUME": "0", "ENOMOTO_MIP_RENS_MAX_DEPTH": "1000", "ENOMOTO_MIP_RENS_LPBC_MAX_DEPTH": "1000"}
CFGS = {"new": {}, "new_t3": T3, "revert": REV}
for p in sys.argv[1].split(","):
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, e in CFGS.items():
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_SUBMIP_VERBOSE="1", **e)
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "60", "--threads", "4"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=300)
        (D / "logs" / f"{p}_{c}.log").write_text(r.stderr + r.stdout)
        res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
        log = r.stderr
        d3 = len(re.findall(r"SMT begin .*depth=3 kind=\[RENS", log))
        print(f"{p:18s} {c:7s} {res['status']:10s} {res['time']:6.1f}s obj={res.get('objective')}  more_nodes={log.count('RENS (more nodes)') - log.count('skipped')} skipped={log.count('RENS (more nodes) skipped')} depth3_RENS={d3} lpbc_d2={len(re.findall(r'SMT begin .*depth=2 kind=\[RENS \(LP before cuts', log))}", flush=True)
