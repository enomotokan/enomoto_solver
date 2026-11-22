"""根だけ (1 スレッド、ENOMOTO_MIP_NODE_LIMIT=1) で既定と ENOMOTO_MIP_OBJSTEPCUT=a・b を比べる。
python rootexp.py TL 問題... >> root.jsonl。ログは logs/PROB_MODE.log.gz。拾うもの: 根の下界、カットのラウンドの数と LP 反復の和 (「cut round」の行)、
外側の根の止まり方 (CUTSTOP、bg でない最後の行)、刻みの行 (OBJSTEP の行の数・最初と最後の zk・lk)、時間"""
import gzip, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = sys.argv[1]
done = set()
if (D / "root.jsonl").exists():
    for l in (D / "root.jsonl").read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["mode"]))
for p in sys.argv[2:]:
    mps = rb.fetch(p, WT / ".miplib_cache")
    for mode in ("0", "a", "b"):
        if (p, mode) in done: continue
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_OBJSTEPCUT": mode, "ENOMOTO_MIP_OBJSTEPCUT_LOG": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1",
                    "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1"})
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", TL, "--threads", "1"],
                            cwd=WT, env=env, capture_output=True, text=True, timeout=int(TL) + 600)
        err = pr.stderr
        with gzip.open(D / "logs" / f"{p}_{mode}.log.gz", "wt") as f: f.write(err)
        try: res = json.loads([l for l in pr.stdout.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash"}  # noqa: BLE001
        rounds = re.findall(r"MIP: cut round (\d+): (\d+) cuts .*?LP (\d+) iters\), LP rows (\d+).*?obj (\S+)", err)
        cs = [json.loads(m) for m in re.findall(r"CUTSTOP (\{.*\})", err)]
        cs = [c for c in cs if not c.get("bg")]
        os_ = re.findall(r"OBJSTEP g (\S+) nnz (\d+) zk (\S+) lk (\S+) restarts (\d+)", err)
        row = {"problem": p, "mode": mode, "status": res.get("status"), "time": res.get("time"), "root_bound": res.get("root_bound"), "bound": res.get("bound"),
               "objective": res.get("objective"), "rounds": len(rounds), "cut_lp_iters": sum(int(r[2]) for r in rounds),
               "cutstop": [(c["reason"], c["restarts"], c["rounds"], c["intobj_stall"], c["obj1"]) for c in cs],
               "objstep": {"n": len(os_), "g": os_[0][0] if os_ else None, "first": os_[0][2:4] if os_ else None, "last": os_[-1][2:4] if os_ else None}}
        with open(D / "root.jsonl", "a") as f: f.write(json.dumps(row) + "\n")
        print(p, mode, row["status"], row["root_bound"], row["rounds"], row["cut_lp_iters"], row["cutstop"][-1:] , row["objstep"], flush=True)
