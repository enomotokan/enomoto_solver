"""カットの種類ごとの効き目の材料集め (P2-INF、2026-11-20): python run_root.py [秒=300] [並べる数=4]
P (MIPLIB の 58 問) と M (Mittelmann 186 問、analysis/vsscip_20261120) の問題を、根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1・
決定的な設定 (ENOMOTO_MIP_TIMEDEP=b・ENOMOTO_MIP_DET1T=onil) で、zero-half を入れて (ENOMOTO_MIP_NO_ZEROHALF=0) 解き、
`ENOMOTO_MIP_SEPSTAT=1` の根の表 (SEPLEARN ... root の行) を読んで root.jsonl に足す (途中から再開できる)。
根の表の行は logs/<問題>.txt に残す (SEPLEARN と root LP の行だけ)。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "300"; par = int(sys.argv[2]) if len(sys.argv) > 2 else 4
def setprobs(name):
    sh = json.loads((WT / f"analysis/vsscip_20261120/shards_{name}.json").read_text())["shards"]
    return sorted({t[0] for s in sh for t in s})
P, M = setprobs("P"), setprobs("M")
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
out = D / "root.jsonl"; (D / "logs").mkdir(exist_ok=True)
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
ENV = {"ENOMOTO_MIP_SEPSTAT": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_NO_ZEROHALF": "0", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_THREADS": "1",
       "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_TIMEDEP": "b", "ENOMOTO_MIP_DET1T": "onil"}
RX = re.compile(r"^SEPLEARN root restarts (\d+) root (\S+)\s+calls (\d+) cands (\d+) chosen (\d+) act1 (\d+) gain (\S+) \(\S+\) surv (\d+)/(\d+) \(\S+\) secs (\S+) ticks (\S+)")
def run(sp):
    S, p = sp
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}; env.update(ENV)
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) + 300)
        err, so = r.stderr, r.stdout
    except subprocess.TimeoutExpired as e:
        err, so = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""), ""
    keep = [l for l in err.splitlines() if l.startswith("SEPLEARN") or l.startswith("MIP: root LP ") or l.startswith("MIP: cut round") and " cuts (of " in l]
    (D / "logs" / f"{p}.txt").write_text("\n".join(keep) + "\n")
    res = {}
    try: res = json.loads([l for l in so.splitlines() if l.startswith("{")][-1])
    except Exception: pass  # noqa: BLE001
    lp0 = next((float(m.group(1)) for l in err.splitlines() if (m := re.match(r"MIP: root LP (\S+) \(", l))), None)
    kinds = {}
    for l in err.splitlines():
        m = RX.match(l)
        if not m: continue
        k = m.group(2); e = kinds.setdefault(k, {"calls": 0, "cands": 0, "chosen": 0, "act1": 0, "gain": 0.0, "surv_ok": 0, "surv_n": 0, "secs": 0.0, "ticks": 0.0})
        for f, v in zip(["calls", "cands", "chosen", "act1"], m.group(3, 4, 5, 6)): e[f] += int(v)
        e["gain"] += float(m.group(7)); e["surv_ok"] += int(m.group(8)); e["surv_n"] += int(m.group(9)); e["secs"] += float(m.group(10)); e["ticks"] += float(m.group(11))
    return {"set": S, "problem": p, "status": res.get("status"), "lp0": lp0, "root_bound": res.get("root_bound"), "objective": res.get("objective"),
            "restarts": max([int(m.group(1)) for l in err.splitlines() if (m := RX.match(l))] or [0]), "kinds": kinds, "wall": round(time.time() - t0, 1)}
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, [sp for sp in probs if sp[1] not in done]):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["set"], row["problem"], row["status"], row["wall"], len(row["kinds"]), flush=True)
