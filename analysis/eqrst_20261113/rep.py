"""uccase12 などを設定ごとに繰り返し解く (4 スレッド)。使い方: python rep.py OUT.jsonl PROBLEM TL N CONFIG[,CONFIG...]
CONFIG: def (既定) / r (EQUIV2_RESTART=1) / rt (r + COEF_TIGHTEN_TINY) / t (TINY のみ)。末尾 f は組の割合の下限 0 (見つけた組を必ず使う)。ログは logs/ に。済んだ分は飛ばす。"""
import json, sys, gzip
from pathlib import Path
ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "benchmarks/psched_ioeq_20261022"))
import run as R
CFG = {"def": {}, "r": {"ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1"},
       "rt": {"ENOMOTO_MIP_PROBE_EQUIV2_RESTART": "1", "ENOMOTO_MIP_COEF_TIGHTEN_TINY": "1"},
       "t": {"ENOMOTO_MIP_COEF_TIGHTEN_TINY": "1"}, "n": {"ENOMOTO_MIP_PROBE_EQUIV2_NORESTART": "1"}}
F0 = {"ENOMOTO_T_MIP_PROBE_EQUIV2_MIN_FRAC": "0"}
CFG.update({k + "f": {**v, **F0} for k, v in list(CFG.items())})
out = Path(sys.argv[1]); prob = sys.argv[2]; tl = float(sys.argv[3]); n = int(sys.argv[4]); cfgs = sys.argv[5].split(",")
logs = out.parent / "logs"; logs.mkdir(exist_ok=True)
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        d = json.loads(l); done.add((d["problem"], d["config"], d["rep"]))
for k in range(n):
    for c in cfgs:
        if (prob, c, k) in done: continue
        r, err = R.run(R.CACHE / f"{prob}.mps", tl, 4, CFG[c], log=True)
        with gzip.open(logs / f"{prob}.{c}.{k}.log.gz", "wt") as f: f.write(err)
        eq = [l.split("implications: ")[1] for l in err.splitlines() if "equivalence pass with implications" in l]
        rl = [l.split("root LP ")[1].split(" ")[0] for l in err.splitlines() if l.startswith("MIP: root LP 1") or l.startswith("MIP: root LP -")]
        sk = sum(int(l.split("skipped ")[1].split(" ")[0]) for l in err.splitlines() if "in rows redundant" in l)
        r.update(problem=prob, config=c, rep=k, eq=eq, root_lps=rl, skipped=sk)
        print(json.dumps({x: r.get(x) for x in ("problem", "config", "rep", "status", "objective", "bound", "time", "eq", "root_lps", "skipped", "feasible")}), flush=True)
        with out.open("a") as f: f.write(json.dumps(r) + "\n")
