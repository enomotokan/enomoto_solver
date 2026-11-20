"""根のヒューリスティクスの走査 (走査だけ): P (MIPLIB 58 問) と M (Mittelmann、P と重ならない 145 問) を、根だけ (ノード上限 1)・1 スレッド・
種 1・TL 秒で回し、根の LP (反復・秒・行・列・非零)、診断 ENOMOTO_MIP_DEBUG_RHSTAT の行 (ヒューリスティクスごとの秒・LP 反復・改善)、
RHEUR の共有予算の行、根のヒューリスティクスの前後の時刻、暫定解の出どころを拾う。
python rhscan.py [TL=600] [並べる数=4] → RHSCAN_OUT (既定 rhscan.jsonl、途中から再開できる)"""
import json, os, re, subprocess, sys, gzip
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
D = Path(__file__).resolve().parent; WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
S = WT / "analysis/scipslow_20261119"
P = sorted({x[0] for s in json.loads((S / "shards_P.json").read_text())["shards"] for x in s})
M = [p for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for p in part]
probs = [("P", p) for p in P] + [("M", p) for p in M if p not in P]
# RHSCAN_PROBS=a,b,c で問題を絞る (組は "X")。RHSCAN_ENV="K=V K=V" で当方に環境変数を足す。RHSCAN_TAG でログの名前に札を付ける
if os.environ.get("RHSCAN_PROBS"):
    probs = [("X", p) for p in os.environ["RHSCAN_PROBS"].split(",")]
TAG = os.environ.get("RHSCAN_TAG", "")
OUT = Path(os.environ.get("RHSCAN_OUT", str(D / "rhscan.jsonl")))
LOGD = D / "logs_rh"; LOGD.mkdir(exist_ok=True)
done = {json.loads(l)["problem"] for l in OUT.read_text().splitlines()} if OUT.exists() else set()
TL = float(sys.argv[1]) if len(sys.argv) > 1 else 600.0
NJ = int(sys.argv[2]) if len(sys.argv) > 2 else 4


def one(sp):
    s, p = sp
    row = {"set": s, "problem": p}
    try:
        mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        row["error"] = str(e)[:200]
        return row
    env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
    env.update({"ENOMOTO_MIP_THREADS": "1", "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1", "ENOMOTO_MIP_DEBUG_RHSTAT": "1"})
    env.update(dict(kv.split("=", 1) for kv in os.environ.get("RHSCAN_ENV", "").split()))
    code = f"""
import time, sys
from pathlib import Path
from enomoto_solver import benchmark_highs as bh
_, lp = bh._load_lp(Path({str(mps)!r}))
m, _, _ = bh._build_our_model(lp)
t = time.perf_counter(); r = m.solve(None, False, {TL}, None, 1, 1, True); t = time.perf_counter() - t
print('RESULT', r['status'], r['objective'], r['best_bound'], r['nodes'], r.get('root_bound'), round(t, 2), file=sys.stderr)
"""
    try:
        pr = subprocess.run([sys.executable, "-c", code], env=env, cwd=WT, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=TL * 1.5 + 300)
        out = pr.stdout.decode(errors="replace")
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode(errors="replace"); row["timeout"] = True
    keep = [l for l in out.splitlines() if l.startswith(("RHSTAT", "RESULT", "MIP: root LP", "MIP: RHEUR", "MIP: after root heuristics", "TRSTAT", "MIP: solving the root LP", "MIP: root heuristic", "MIP: RHBUDGET"))]
    (LOGD / f"{p}{TAG}.log.gz").write_bytes(gzip.compress(("\n".join(keep) + "\n").encode()))
    m = re.search(r"MIP: root LP (\S+) \((\d+) iters, ([\d.]+)s\)", out)
    if m: row["root_lp"] = [float(m.group(1)), int(m.group(2)), float(m.group(3))]
    m = re.search(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", out)
    if m: row["root_size"] = [int(m.group(1)), int(m.group(2)), int(m.group(3))]
    m = re.search(r"MIP: solving the root LP \(([\d.]+)s\)", out)
    if m: row["t_root_start"] = float(m.group(1))
    m = re.search(r"MIP: after root heuristics: incumbent (\S+) \(([\d.]+)s\)", out)
    if m: row["after_heur"] = [m.group(1), float(m.group(2))]
    row["rhstat"] = [l.split()[1:] for l in out.splitlines() if l.startswith("RHSTAT")]
    row["rheur"] = [l for l in out.splitlines() if l.startswith("MIP: RHEUR")]
    m = re.search(r"RESULT (\S+) (\S+) (\S+) (\S+) (\S+) (\S+)", out)
    if m: row["result"] = list(m.groups())
    return row


todo = [sp for sp in probs if sp[1] not in done]
with ThreadPoolExecutor(NJ) as ex, OUT.open("a") as f:
    for row in ex.map(one, todo):
        f.write(json.dumps(row, ensure_ascii=False) + "\n"); f.flush()
        print(row["problem"], row.get("result", ["?"])[0], row.get("root_lp"), len(row.get("rhstat", [])), flush=True)
