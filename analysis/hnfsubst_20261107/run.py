"""既定 (def)・HNFPRE=1 (hnf)・HNFSUBST=1 (sub) などを 4 スレッドで解く (2026-11-07)。1 回ずつ順に (ソルバーの計測は同時に 1 つ)。
使い方: python run.py out.jsonl T cfgs(カンマ区切り) seeds(カンマ区切り) 問題...  (@hits で合同式の出る問題、@p58 で 58 問)。
既に out にある (問題, 設定, 回) は飛ばす (止まったら同じ命令で続きから回る)。標準エラーは logs/ に gz で置き、根の値・カットを抜き出す。"""
import gzip, json, os, re, subprocess, sys, time
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
HITS = ["flugpl", "flugplinf", "brazil3", "lectsched-5-obj", "mzzv11", "neos-3656078-kumeu", "piperout-08", "piperout-27",
        "radiationm18-12-05", "radiationm40-10-02", "rocI-4-11", "rococoB10-011000", "rococoC10-001000", "snp-02-004-104",
        "traininstance2", "traininstance6", "8div-n59k10"]
CFG = {"def": {}, "hnf": {"ENOMOTO_MIP_HNFPRE": "1"}, "sub": {"ENOMOTO_MIP_HNFSUBST": "1"},
       "subh": {"ENOMOTO_MIP_HNFSUBST": "1", "ENOMOTO_MIP_HNFPRE": "1"}, "cut": {"ENOMOTO_MIP_HNFCUT": "1"}}
out = Path(sys.argv[1]); T = float(sys.argv[2]); cfgs = sys.argv[3].split(","); seeds = [int(s) for s in sys.argv[4].split(",")]
probs = []
for a in sys.argv[5:]:
    if a == "@hits":
        probs += HITS
    elif a == "@p58":
        import probs as P58
        probs += list(P58.PROBS) if hasattr(P58, "PROBS") else list(P58.probs)
    else:
        probs.append(a)
(D / "logs").mkdir(exist_ok=True)
done = set()
if out.exists():
    for l in out.open():
        try:
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["rep"]))
        except Exception:
            pass

def parse(err):
    o = {}
    m = re.search(r"MIP: root LP ([-+0-9.e]+) \(", err)
    if m: o["root_lp"] = float(m.group(1))
    m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP, pool (\d+) cuts .*obj ([-+0-9.e]+) -> ([-+0-9.e]+)", err)
    if m: o.update(cuts_added=int(m.group(1)), cuts_lp=int(m.group(2)), root_after_cuts=float(m.group(5)))
    m = re.search(r"SEPPROF restarts 0   cands/chosen per round:(.*)", err)
    if m: o["chosen"] = m.group(1).strip()
    m = re.search(r"SEPPROF restarts 0 rounds (\d+)", err)
    if m: o["rounds"] = int(m.group(1))
    m = re.search(r"HNFSUBST: substituted (\d+) of (\d+)", err)
    if m: o["subst"] = int(m.group(1))
    o["restarts"] = len(re.findall(r"^MIP: restart \d+ \(", err, re.M))
    return o

for s in seeds:
    for p in probs:
        for c in cfgs:
            if (p, c, s) in done:
                continue
            env = dict(os.environ, ENOMOTO_MIP_LOG="1", ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(s), ENOMOTO_MIP_HNFPRE_QUIET="1",
                       ENOMOTO_MIP_DEBUG_SEPPROF="1", **CFG[c])
            mps = WT / ".miplib_cache" / f"{p}.mps"
            if not mps.exists():
                subprocess.run([sys.executable, "-c", f"import sys; sys.path.insert(0,'{WT}/scripts'); import run_miplib_benchmark as r; from pathlib import Path; r.fetch('{p}', Path('{WT}/.miplib_cache'))"])
            t0 = time.time()
            try:
                pr = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", str(T), "--threads", "4"],
                                    capture_output=True, text=True, env=env, timeout=T * 3 + 120)
                out_s, err = pr.stdout, pr.stderr
            except subprocess.TimeoutExpired as e:
                out_s, err = "", (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or "")
            try:
                res = json.loads(out_s.strip().splitlines()[-1])
            except Exception:
                res = {"status": "error", "err": err[-500:]}
            with gzip.open(D / "logs" / f"{out.stem}.{p}.{c}.{s}.log.gz", "wt") as f:
                f.write(err)
            rec = {"problem": p, "cfg": c, "rep": s, "T": T, "result": res, "info": parse(err), "wall": round(time.time() - t0, 2)}
            with out.open("a") as f:
                f.write(json.dumps(rec) + "\n")
            print(p, c, s, res.get("status"), res.get("time"), res.get("nodes"), rec["info"].get("root_after_cuts"), flush=True)
