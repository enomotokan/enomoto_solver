"""木の中の分離を深さ別に見る計測 (analysis/treedepth_20261121/README.md)。ソルバーは同時に 1 つ。途中から再開できる。
使い方: python run.py OUT.jsonl 計画,...   計画 = スレッド数:種 (例 4:1,4:2,1:1)
M の問題は 600 秒、P の問題は 60 秒。診断 `ENOMOTO_MIP_TREEDEPTH_DIAG` の生の記録は logs/<OUT>_<問題>_t<スレッド>_s<種>.jsonl.gz に残す。"""
import gzip, json, os, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = "neos-911970 misc07 neos5 mik-250-20-75-4 noswot".split()
M = "fiball csched008 neos-3627168-kasai neos-957323 mzzv11 tr12-30 uccase12".split()
TL = {**{p: 60 for p in P}, **{p: 600 for p in M}}
OUT = Path(sys.argv[1])
plans = [tuple(int(v) for v in x.split(":")) for x in sys.argv[2].split(",")]
only = set(sys.argv[3].split(",")) if len(sys.argv) > 3 else None
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["threads"], r["rep"]))
(D / "logs").mkdir(exist_ok=True)
for group in (P, M):
    for th, s in plans:
        for p in group:
            if (p, th, s) in done or (only and p not in only):
                continue
            mps = rb.fetch(p, WT / ".miplib_cache")
            raw = D / "logs" / f"{OUT.stem}_{p}_t{th}_s{s}.jsonl"
            raw.unlink(missing_ok=True)
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            env.update({"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_THREADS": str(th), "ENOMOTO_T_MIP_SEED": str(s), "ENOMOTO_MIP_TREEDEPTH_DIAG": str(raw)})
            t0 = time.time()
            err = ""
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL[p]), "--threads", str(th)],
                                    cwd=WT, env=env, capture_output=True, text=True, timeout=TL[p] + 300)
                out, err = pr.stdout, pr.stderr
                res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[-200:]}
            keep = [l for l in err.splitlines() if l.startswith(("MIP: node diag", "MIP: adaptive", "MIP: tree search"))]
            (D / "logs" / f"{OUT.stem}_{p}_t{th}_s{s}.log").write_text("\n".join(keep) + "\n")
            if raw.exists():
                with open(raw, "rb") as f, gzip.open(str(raw) + ".gz", "wb") as g:
                    g.write(f.read())
                raw.unlink()
            r = {"problem": p, "threads": th, "rep": s, "tl": TL[p], "result": res, "wall": round(time.time() - t0, 1)}
            with OUT.open("a") as f:
                f.write(json.dumps(r) + "\n")
            print(p, th, s, res.get("status"), res.get("time"), res.get("bound"), res.get("objective"), res.get("nodes"), flush=True)
