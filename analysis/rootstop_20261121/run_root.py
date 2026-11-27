"""根だけの計測 (analysis/sepalloc_20261121/run_root.py を元に): python run_root.py 設定 [並べる数=3] [問題の名前のファイル]
P (MIPLIB の 58 問、上限 60 秒) と M (Mittelmann、P と重ならない 145 問、上限 600 秒) を、根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1・
既定の決定的な設定 (根の上限は手間で測る) で解き、CUTSTOP (止めた理由)・CUTTK (巡ごとの初めの下界と手間)・巡の行を root_<設定>.jsonl に足す
(途中から再開できる)。上限の秒は本番と同じ (根のカットの時間の枠が時間の上限の割合で決まるため)。設定は cfg.py。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(D)); sys.path.insert(0, str(WT / "scripts"))
from cfg import CFGS  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
TAG = sys.argv[1]; par = int(sys.argv[2]) if len(sys.argv) > 2 else 3
ONLY = set(open(sys.argv[3]).read().split()) if len(sys.argv) > 3 else None
def setprobs(name):
    sh = json.loads((WT / f"analysis/vsscip_20261120/shards_{name}.json").read_text())["shards"]
    return sorted({t[0] for s in sh for t in s})
P, M = setprobs("P"), setprobs("M")
probs = [("P", p, 60) for p in P] + [("M", p, 600) for p in M if p not in P]
if ONLY is not None: probs = [sp for sp in probs if sp[1] in ONLY]
out = D / f"root_{TAG}.jsonl"
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
ENV = {"ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_CUTSTOP_LOG": "1"}
RD = re.compile(r"MIP: cut round (\d+): (\d+) cuts \(of (\d+), sep ([\d.]+)s, LP (\d+) iters\), LP rows (\d+) \(cut rows mean nnz (\d+)\), obj (\S+) \(([\d.]+)s\)")
def run(sp):
    S, p, tl = sp
    for _ in range(5):
        try: mps = rb.fetch(p, WT / ".miplib_cache"); break
        except Exception: time.sleep(30)  # noqa: BLE001
    else:
        return {"set": S, "problem": p, "status": "fetch_error"}
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}; env.update(ENV); env.update(CFGS[TAG])
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(tl), "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 600)
        err, so = r.stderr, r.stdout
    except subprocess.TimeoutExpired as e:
        err = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); so = ""
    res = {}
    try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
    except Exception: pass  # noqa: BLE001
    L = err.splitlines()
    lp0 = next((float(m.group(1)) for l in L if (m := re.match(r"MIP: root LP (\S+) \(", l))), None)
    rows0 = next((int(m.group(1)) for l in L if (m := re.search(r"root LP size: rows (\d+)", l))), None)
    cs = [c for c in (json.loads(l[len("CUTSTOP "):]) for l in L if l.startswith("CUTSTOP {")) if not c.get("bg")]
    tk = [(int(m.group(1)), float(m.group(2)), float(m.group(3))) for l in L if (m := re.match(r"CUTTK round (\d+) obj (\S+) tk (\S+)", l))]
    rd = [[int(m.group(i)) for i in (1, 2, 3, 5, 6, 7)] + [float(m.group(4)), float(m.group(8)), float(m.group(9))] for l in L if (m := RD.search(l))]
    rd_end = next((i for i, l in enumerate(L) if "cut loop done" in l), None)
    return {"set": S, "problem": p, "tl": tl, "status": res.get("status"), "lp0": lp0, "rows0": rows0, "root_bound": res.get("root_bound"),
            "objective": res.get("objective"), "time": res.get("time"), "cutstops": cs, "cuttk": tk[:3000],
            "rounds": rd[:3000], "nrounds_first": sum(1 for i, l in enumerate(L) if rd_end is not None and i < rd_end and RD.search(l)),
            "wall": round(time.time() - t0, 1)}
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, [sp for sp in probs if sp[1] not in done]):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["set"], row["problem"], row["status"], row.get("wall"), flush=True)
