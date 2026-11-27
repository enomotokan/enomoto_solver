"""1 スレッド・種 1 の通し (決定的) で既定 (def) と ENOMOTO_MIP_PROBREP=1 (pr) を比べる。python compare.py TL 問題... >> compare.jsonl
拾うもの: 状態・秒・ノード・目的値・下界、根の LP を解き始めた時刻 (前処理の秒の目安)、プロービングの回数と秒、PROBREP で省いた回数"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = sys.argv[1]
OUT = os.environ.get("PROBREP_OUT", "compare")  # 出力の名 (2 版目のビルドは compare2)
RX = re.compile(r"MIP: probing: \d+ of \d+ binaries probed .*?, (\d+) fixed, (\d+) bounds tightened, \d+ implications \(([0-9.]+)s")
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for c, extra in (("def", {}), ("pr", {"ENOMOTO_MIP_PROBREP": "1"})):
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"}); env.update(extra)
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=int(TL) + 600)
        err = pr.stderr
        with gzip.open(D / "logs" / f"{p}_{OUT}_{c}.log.gz", "wt") as f: f.write(err)
        res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        pb = [(int(a), int(b), float(t)) for a, b, t in RX.findall(err)]
        root = re.findall(r"MIP: solving the root LP \(([0-9.]+)s\)", err)
        row = {"problem": p, "cfg": c, "status": res.get("status"), "time": res.get("time"), "nodes": res.get("nodes"), "objective": res.get("objective"), "bound": res.get("bound"),
               "root_start": float(root[0]) if root else None, "probes": len(pb), "probe_fixed": sum(x[0] for x in pb), "probe_secs": round(sum(x[2] for x in pb), 2),
               "skipped": err.count("skipped (PROBREP")}
        with open(D / f"{OUT}.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(p, c, row["status"], round(row["time"] or 0, 1), row["nodes"], row["root_start"], row["probes"], row["probe_secs"], row["skipped"], flush=True)
