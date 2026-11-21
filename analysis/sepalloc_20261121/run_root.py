"""伸びの取り分で配る試作 ENOMOTO_MIP_SEPALLOC の根だけの比べ (../sepeff_20261120/run_root.py を元に、SA_OUT・SA_ENV・SA_PROBS で名前・足す環境変数・問題を変える) (P2-INF、2026-11-21): python run_root.py [秒=300] [並べる数=4]
P (MIPLIB の 58 問) と M (Mittelmann 186 問、analysis/vsscip_20261120) の問題を、根だけ (ENOMOTO_MIP_NODE_LIMIT=1)・1 スレッド・種 1・
決定的な設定 (ENOMOTO_MIP_TIMEDEP=b・ENOMOTO_MIP_DET1T=onil) で、zero-half を入れて (ENOMOTO_MIP_NO_ZEROHALF=0) 解き、
`ENOMOTO_MIP_SEPSTAT=1` の根の表 (SEPLEARN ... root の行) を読んで root.jsonl に足す (途中から再開できる)。
根の表の行は logs/<問題>.txt に残す (SEPLEARN と root LP の行だけ)。"""
import json, os, re, subprocess, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
tl = sys.argv[1] if len(sys.argv) > 1 else "300"; par = int(sys.argv[2]) if len(sys.argv) > 2 else 4
# SA_OUT (結果の名前、既定 def)・SA_ENV (足す環境変数 "A=1,B=2")・SA_PROBS (問題をこのファイルの行に絞る)
TAG = os.environ.get("SA_OUT", "def")
EXTRA = dict(kv.split("=", 1) for kv in os.environ.get("SA_ENV", "").split(",") if "=" in kv)
ONLY = set(open(os.environ["SA_PROBS"]).read().split()) if os.environ.get("SA_PROBS") else None
def setprobs(name):
    sh = json.loads((WT / f"analysis/vsscip_20261120/shards_{name}.json").read_text())["shards"]
    return sorted({t[0] for s in sh for t in s})
P, M = setprobs("P"), setprobs("M")
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
if ONLY is not None: probs = [sp for sp in probs if sp[1] in ONLY]
out = D / f"{TAG}.jsonl"; LOGD = D / f"logs_{TAG}"; LOGD.mkdir(exist_ok=True)
done = {json.loads(l)["problem"] for l in open(out)} if out.exists() else set()
ENV = {"ENOMOTO_MIP_SEPSTAT": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_NO_ZEROHALF": "0", "ENOMOTO_MIP_NODE_LIMIT": "1", "ENOMOTO_MIP_THREADS": "1",
       "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_TIMEDEP": "b", "ENOMOTO_MIP_DET1T": "onil", "ENOMOTO_MIP_CUTSTOP_LOG": "1"}
RX = re.compile(r"^SEPLEARN root restarts (\d+) root (\S+)\s+calls (\d+) cands (\d+) chosen (\d+) act1 (\d+) gain (\S+) \(\S+\) surv (\d+)/(\d+) \(\S+\) secs (\S+) ticks (\S+)")
def run(sp):
    S, p = sp
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}; env.update(ENV); env.update(EXTRA)
    t0 = time.time()
    try:
        r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(WT / ".miplib_cache" / f"{p}.mps"), "--time-limit", tl, "--threads", "1"],
                           cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) + 300)
        err, so = r.stderr, r.stdout
    except subprocess.TimeoutExpired as e:
        err, so = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""), ""
    keep = [l for l in err.splitlines() if l.startswith("SEPLEARN") or l.startswith("SEPEFF") or l.startswith("CUTSTOP") or l.startswith("CUTTK") or l.startswith("SEPALLOC") or "SEPALLOC s:" in l or l.startswith("MIP: root LP ") or l.startswith("MIP: cut round") and " cuts (of " in l]
    (LOGD / f"{p}.txt").write_text("\n".join(keep) + "\n")
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
    face = next((dict((k, float(v)) for k, v in re.findall(r"(\w+) ([-0-9.e]+)", l[len("SEPEFF face "):])) for l in err.splitlines() if l.startswith("SEPEFF face ")), None)
    fc = next((l for l in err.splitlines() if l.startswith("SEPEFF facecut")), None)
    fcd = None
    if fc and (m := re.search(r"obj (\S+) -> (\S+) \(gain (\S+), of gap (\S+)\).*?chosen (\d+) \((\d+) cut x\*\), ticks (\S+), (\S+)s", fc)):
        fcd = {"obj0": float(m.group(1)), "obj1": float(m.group(2)), "gain": float(m.group(3)), "gapfrac": None if m.group(4) == "-" else float(m.group(4)), "chosen": int(m.group(5)), "cut_x": int(m.group(6)), "ticks": float(m.group(7)), "secs": float(m.group(8))}
    rounds = [float(m.group(1)) for l in err.splitlines() if (m := re.match(r"MIP: cut round \d+: \d+ cuts \(of .*obj (\S+) \(", l))]
    cs = [json.loads(l[len("CUTSTOP "):]) for l in err.splitlines() if l.startswith("CUTSTOP {")]
    cs = [c for c in cs if not c.get("bg")]
    tk = [(int(m.group(1)), float(m.group(2)), float(m.group(3))) for l in err.splitlines() if (m := re.match(r"CUTTK round (\d+) obj (\S+) tk (\S+)", l))]
    sa = [l for l in err.splitlines() if l.startswith("SEPALLOC rounds")]
    return {"cutstops": cs, "cuttk": tk[:2000], "sepalloc": sa, "cutstop": cs[0] if cs else None, "face": face, "facecut": fcd, "facecut_line": fc, "round_obj": rounds[:400], "set": S, "problem": p, "status": res.get("status"), "lp0": lp0, "root_bound": res.get("root_bound"), "objective": res.get("objective"),
            "restarts": max([int(m.group(1)) for l in err.splitlines() if (m := RX.match(l))] or [0]), "kinds": kinds, "wall": round(time.time() - t0, 1)}
with ThreadPoolExecutor(par) as ex:
    for row in ex.map(run, [sp for sp in probs if sp[1] not in done]):
        with open(out, "a") as f: f.write(json.dumps(row) + "\n")
        print(row["set"], row["problem"], row["status"], row["wall"], len(row["kinds"]), flush=True)
