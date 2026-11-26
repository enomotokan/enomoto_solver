"""OBJSHAVE と既定の下界の推移の比べ: 候補 (見積りの 4 問 + 下界だけ残る 22 問) を 4 スレッド・種 1・TL 秒 (既定 300) で解く。
各問題で先に shave を回し、ログに「OBJSHAVE: started」が無ければ (目的が整数の刻みを取らないなど、試作が働かない = 既定と同じ設定) 既定は回さない。
ログからは、進みの行 (「MIP: 秒 nodes … lb … ub」) と、OBJSHAVE の受け取りの行 (「lower bound … received (秒」) を拾う。
python run.py → res.jsonl (途中から再開できる)。ソルバーは同時に 1 つ。PYTHONPATH は呼ぶ側で渡す"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
TL = int(os.environ.get("RUN_TL", "300"))
pick = json.loads((D / "pick.json").read_text())
solu = {}
for l in (WT / "scripts/miplib2017-v31.solu").read_text().splitlines():
    t = l.split()
    if len(t) >= 3 and t[0] == "=opt=": solu[t[1]] = float(t[2])
bnd = [p for p, c, d in pick if c == "def_t4" and d and d["opt"] is not None and d["zgap"] is not None and abs(d["zgap"]) < 1e-6]
PROBS = ["lectsched-5-obj", "net12", "n3div36", "glass-sc"] + [p for p in bnd if p not in ("net12", "glass-sc")]
OUT = D / "res.jsonl"; LOGS = D / "logs"; LOGS.mkdir(exist_ok=True)
done = set()
if OUT.exists():
    for l in OUT.read_text().splitlines():
        r = json.loads(l); done.add((r["problem"], r["cfg"]))
PROG = re.compile(r"^MIP: +([\d.]+)s nodes +\d+ open +\d+ lb (\S+) ub (\S+)")
RECV = re.compile(r"OBJSHAVE: lower bound (\S+) received \(([\d.]+)s")
def run(p, cfg):
    mps = rb.fetch(p, WT / ".miplib_cache")
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "4", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
    if cfg == "shave": env["ENOMOTO_MIP_OBJSHAVE"] = "1"
    t0 = time.time()
    pr = subprocess.Popen([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(TL), "--threads", "4"],
                          cwd=WT, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try: out, err = pr.communicate(timeout=TL + 600)
    except subprocess.TimeoutExpired:
        pr.kill(); out, err = pr.communicate()
    try: res = json.loads([l for l in out.splitlines() if l.startswith("{")][-1])
    except Exception: res = {"status": "crash", "returncode": pr.returncode}  # noqa: BLE001
    prog, recv, keep = [], [], []
    for l in err.splitlines():
        m = PROG.match(l)
        if m: prog.append([float(m[1]), float(m[2]), float(m[3])]); keep.append(l); continue
        m = RECV.search(l)
        if m: recv.append([float(m[2]), float(m[1])])
        if "OBJSHAVE" in l or l.startswith("TRSTAT: tree start"): keep.append(l)
    (LOGS / f"{p}_{cfg}.log.gz").write_bytes(gzip.compress(("\n".join(keep) + "\n").encode()))
    return {"problem": p, "cfg": cfg, "tl": TL, "res": res, "prog": prog, "recv": recv, "active": any("OBJSHAVE: started" in l for l in keep),
            "opt": solu.get(p), "wall": round(time.time() - t0, 1)}
for p in PROBS:
    for cfg in ("shave", "def"):
        if (p, cfg) in done: continue
        if cfg == "def":
            rs = [json.loads(l) for l in OUT.read_text().splitlines()] if OUT.exists() else []
            sh = [r for r in rs if r["problem"] == p and r["cfg"] == "shave"]
            if sh and not sh[0]["active"]:
                continue
        r = run(p, cfg)
        with open(OUT, "a") as f: f.write(json.dumps(r) + "\n")
        print(json.dumps({k: r[k] for k in ("problem", "cfg", "active")} | {"status": r["res"].get("status"), "bound": r["res"].get("bound")}), flush=True)
print("RUN_DONE", flush=True)
