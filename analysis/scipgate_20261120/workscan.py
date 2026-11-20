"""試作 ENOMOTO_MIP_VBCLQ が働く問題 (根の LP の前に pseudoobj が固定する問題) を、P 58 問 + M (P と重ならない問題) で洗い出す。
VB01=b (SCIP-VB の根の前の vbounds) + VBCLQ=1 (LP probing は既定どおり行い、その固定の数も拾う)、1 スレッド・nice 19・1 問 120 秒まで、ENOMOTO_MIP_LOG=1 のログから VB01・VBCLQ の行を拾う。
python workscan.py K NSHARD → workscan.jsonl_K (途中から再開できる)"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
P = sorted({x[0] for s in json.loads((WT / "analysis/scipslow_20261119/shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
k, ns = int(sys.argv[1]), int(sys.argv[2])
OUT = Path(str(D / "workscan.jsonl") + f"_{k}")
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
for s, p in probs[k::ns]:
    if p in done: continue
    row = {"set": s, "problem": p}
    try:
        had = (WT / ".miplib_cache" / f"{p}.mps").exists()
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
    else:
        env = {kk: v for kk, v in os.environ.items() if not kk.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_VB01": "b", "ENOMOTO_MIP_VBCLQ": "1", "ENOMOTO_T_MIP_SEED": "1"})
        try:
            pr = subprocess.run(["nice", "-n", "19", sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", "120", "--threads", "1"],
                                cwd=WT, env=env, capture_output=True, text=True, timeout=400)
            out = pr.stderr + pr.stdout
        except subprocess.TimeoutExpired as e:
            out = (e.stderr or b"").decode(errors="replace") if isinstance(e.stderr, bytes) else (e.stderr or ""); row["timeout"] = True
        m = re.findall(r"VBCLQ: pseudo-objective with cliques: (\d+) objective columns substituted, (\d+) fixed in (\d+) passes(, no solution better than the cutoff)?, slack (\S+), free integers now (\d+)", out)
        row["vbclq"] = [{"subst": int(a), "fixed": int(b), "passes": int(c), "proved": bool(d), "slack": float(e), "free": int(f)} for a, b, c, d, e, f in m]
        row["vb01"] = re.findall(r"VB01 before the root LP: (\w+)", out)
        row["root_lp"] = re.findall(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", out)[:1]
        row["lp_probe"] = re.findall(r"LP probing: root LP ([\d.]+)s \((\d+) iters", out)[:1]
        # 前処理の LP による固定 (`l` で省く仕事): 試した列・固定した列 (1 回目の求解の分。再スタートの後は LP probing をしない)
        row["lp_probe_fix"] = [(int(a), int(b)) for a, b in re.findall(r"MIP: LP probing: tried (\d+) columns, fixed (\d+)", out)][:1]
        res = [l for l in out.splitlines() if l.startswith("{")]
        row["result"] = json.loads(res[-1]) if res else None
        if not had: mps.unlink()
    with open(OUT, "a") as f: f.write(json.dumps(row) + "\n")
    print(json.dumps({kk: v for kk, v in row.items() if kk in ("set", "problem", "vbclq", "vb01")}), flush=True)
