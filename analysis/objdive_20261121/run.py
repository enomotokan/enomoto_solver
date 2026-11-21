"""目的を外したダイビング (ENOMOTO_MIP_OBJDIVE) とダイビングの反復の配り方 (ENOMOTO_MIP_DIVEQUOTA) の試作の比べ (analysis/objdive_20261121)。
使い方 (どこからでも): python analysis/objdive_20261121/run.py SET
  SET = A … kakapo・brazil3・momentum1 × 種 1・2 × CFGS、300 秒・4 スレッド
  SET = B … 暫定解の遅い問題 (時間切れで暫定解が SCIP・他の設定より大きく悪かったもの) × 種 1 × CFGS、300 秒・4 スレッド
結果は res_SET.jsonl に足す (途中から再開できる)。各行の result.hist が暫定解の推移 (ログの new incumbent)、
result.dive が主スレッドの最後の状況行の (lp_iters, dive_iters)、result.objdive が目的を外したダイビングの (回数, 整数に達した回数, 解を受け入れた回数)。
ログは logs/<問題>_<設定>_s<種>.log.gz。ソルバーは同時に 1 つ。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "dqs": {"ENOMOTO_MIP_DIVEQUOTA": "s"},
    "dqe": {"ENOMOTO_MIP_DIVEQUOTA": "e"},
    "ods": {"ENOMOTO_MIP_OBJDIVE": "1", "ENOMOTO_MIP_DIVEQUOTA": "s"},
}
SETS = {
    "A": (["neos-5107597-kakapo", "brazil3", "momentum1"], (1, 2)),
    "B": (["dws008-01", "rococoB10-011000", "traininstance2", "academictimetablesmall", "cost266-UUE"], (1,)),
}
TL = 300
SET = sys.argv[1]
probs, seeds = SETS[SET]
OUT = D / f"res_{SET}.jsonl"
jobs = [(p, s, c) for s in seeds for p in probs for c in CFGS]
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        if l.strip():
            r = json.loads(l)
            done.add((r["problem"], r["seed"], r["cfg"]))
INC = re.compile(r"MIP: new incumbent (\S+) at node (\d+) \(([\d.]+)s\) \[(.*)\]")
STAT = re.compile(r"lp_iters (\d+) .*dive_iters (\d+)")
OD = re.compile(r"objective pseudocost dive: (.*?) \(found (true|false)")
for p, s, c in jobs:
    if (p, s, c) in done:
        continue
    mps = rb.fetch(p, WT / ".miplib_cache")
    t0 = time.time()
    log = D / "logs" / f"{p}_{c}_s{s}.log"
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update(ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s), ENOMOTO_MIP_DEBUG_OBJDIVE="1", **CFGS[c])
    with open(log, "w") as f:
        pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                            cwd=WT, env=env, stdout=f, stderr=subprocess.STDOUT, timeout=TL + 600)
    txt = log.read_text(errors="replace")
    try:
        res = json.loads([l for l in txt.splitlines() if l.startswith("{")][-1])
    except Exception:  # noqa: BLE001
        res = {"status": "crash", "returncode": pr.returncode}
    res["hist"] = [{"t": float(m.group(3)), "z": float(m.group(1)), "src": m.group(4), "node": int(m.group(2))} for m in INC.finditer(txt)]
    st = STAT.findall(txt)
    res["dive"] = [int(st[-1][0]), int(st[-1][1])] if st else None
    od = OD.findall(txt)
    res["objdive"] = [len(od), sum(1 for w, _ in od if w == "integral"), sum(1 for _, f in od if f == "true")]
    with open(log, "rb") as fi, gzip.open(str(log) + ".gz", "wb") as fo:
        fo.write(fi.read())
    log.unlink()
    with open(OUT, "a") as f:
        f.write(json.dumps({"problem": p, "seed": s, "cfg": c, "tl": TL, "result": res, "wall": round(time.time() - t0, 1), "at": time.strftime("%H:%M:%S")}) + "\n")
    print(p, s, c, res.get("status"), res.get("objective"), len(res["hist"]), res["dive"], res["objdive"], flush=True)
