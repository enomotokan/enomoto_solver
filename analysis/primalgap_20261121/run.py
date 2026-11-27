"""時間切れのときの暫定解が SCIP より大きく悪い問題の切り分け (診断だけ、既定は変えない)。
使い方 (どこからでも): python analysis/primalgap_20261121/run.py O   … 当方 (4 スレッド既定、種 1〜4、600 秒、ENOMOTO_MIP_LOG=1)
                       python analysis/primalgap_20261121/run.py F   … 直し (ENOMOTO_MIP_SUBMIP_FEASTOL) の確かめ: momentum1 を def (既定) と sft0 (=0、以前の動き) で、
                                                                      4 スレッド・種 1〜4・600 秒 (res_F.jsonl、ログ logs/momentum1_<設定>_s<種>.log.gz)
                       python analysis/primalgap_20261121/run.py S   … SCIP (1 スレッド、種 1〜2、600 秒、解が見つかるたびに時刻・値・ヒューリスティクスを記録し、統計を残す)
結果は res_O.jsonl・res_S.jsonl に足す (途中から再開できる)。ログは logs/<問題>_<ours|scip>_s<種>.log.gz。ソルバーは同時に 1 つ。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
PROBS = ["brazil3", "momentum1", "neos-5107597-kakapo", "sorrell3"]
TL = 600
SET = sys.argv[1]
if SET == "SCIPWORKER":  # 子プロセス: SCIP を 1 回解く
    import pyscipopt
    from pyscipopt import Eventhdlr, SCIP_EVENTTYPE
    mps, seed, statf = sys.argv[2], int(sys.argv[3]), sys.argv[4]
    m = pyscipopt.Model()
    m.setParam("limits/time", TL); m.setParam("limits/gap", 0.0)
    m.setParam("randomization/randomseedshift", seed)
    m.readProblem(mps)
    hist = []
    class H(Eventhdlr):
        def eventinit(self): self.model.catchEvent(SCIP_EVENTTYPE.BESTSOLFOUND, self)
        def eventexit(self): self.model.dropEvent(SCIP_EVENTTYPE.BESTSOLFOUND, self)
        def eventexec(self, ev):
            s = self.model.getBestSol()  # pyscipopt 6.3 には解を出したヒューリスティクスを返す関数がないので、出どころはログの行頭の文字と統計で見る
            hist.append({"t": round(self.model.getSolvingTime(), 2), "z": self.model.getSolObjVal(s), "node": self.model.getNNodes(), "depth": self.model.getDepth()})
    m.includeEventhdlr(H(), "inclog", "incumbent log")
    t = time.perf_counter(); m.optimize(); t = time.perf_counter() - t
    m.writeStatistics(statf)
    has = m.getNSols() > 0
    print(json.dumps({"status": rb.SCIP_STATUS.get(m.getStatus(), m.getStatus()), "objective": m.getObjVal() if has else None,
                      "bound": m.getDualbound(), "nodes": m.getNTotalNodes(), "time": t, "hist": hist}))
    sys.exit(0)
OUT = D / f"res_{SET}.jsonl"
CFGS = {"ours": {}, "def": {}, "sft0": {"ENOMOTO_MIP_SUBMIP_FEASTOL": "0"}}
if SET == "F":
    jobs = [("momentum1", s, c) for s in (1, 2, 3, 4) for c in (("def", "sft0") if s % 2 else ("sft0", "def"))]
else:
    jobs = [(p, s, "ours" if SET == "O" else "scip") for s in ((1, 2, 3, 4) if SET == "O" else (1, 2)) for p in PROBS]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip(): r = json.loads(l); done.add((r["problem"], r["seed"], r.get("cfg", "ours" if SET == "O" else "scip")))
INC = re.compile(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]")
for p, s, cfg in jobs:
    if (p, s, cfg) in done: continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    t0 = time.time()
    if SET in ("O", "F"):
        log = D / "logs" / f"{p}_{cfg}_s{s}.log"
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update(CFGS[cfg])
        env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s))
        with open(log, "w") as f:
            pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                                cwd=WT, env=env, stdout=f, stderr=subprocess.STDOUT, timeout=TL + 600)
        txt = log.read_text(errors="replace")
        try: res = json.loads([l for l in txt.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        res["hist"] = [{"t": float(m.group(3)), "z": float(m.group(1)), "src": m.group(4), "node": int(m.group(2))} for m in INC.finditer(txt)]
        with open(log, "rb") as fi, gzip.open(str(log) + ".gz", "wb") as fo: fo.write(fi.read())
        log.unlink()
    else:
        statf = D / "logs" / f"{p}_scip_s{s}.stat"
        log = D / "logs" / f"{p}_scip_s{s}.log"
        with open(log, "w") as f:
            pr = subprocess.run([sys.executable, __file__, "SCIPWORKER", str(mps), str(s), str(statf)], cwd=WT, stdout=f, stderr=subprocess.STDOUT, text=True, timeout=TL + 600)
        txt = log.read_text(errors="replace")
        try: res = json.loads([l for l in txt.splitlines() if l.startswith("{")][-1])
        except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
        for g in (statf, log):
            if g.exists():
                with open(g, "rb") as fi, gzip.open(str(g) + ".gz", "wb") as fo: fo.write(fi.read())
                g.unlink()
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "seed": s, "cfg": cfg, "set": SET, "tl": TL, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, s, cfg, res.get("status"), res.get("objective"), len(res.get("hist", [])), flush=True)
