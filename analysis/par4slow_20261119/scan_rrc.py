"""根の被約費用のヒューリスティクスの固定率と、伝播の後に自由に残る列の割合を集める: python scan_rrc.py スレッド数 秒 [M|P|問題 ...]。
既定の設定で ENOMOTO_MIP_LOG=1 をつけて「秒」で打ち切り、"root reduced cost: ... free columns" の行を scanrrc_tT.jsonl に足す (途中から再開できる)。
M は Mittelmann 186 問、P は MIPLIB の 58 問。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "scripts"))
import run_miplib_benchmark as rb  # noqa: E402
th, tl = sys.argv[1], sys.argv[2]
sys.path.insert(0, str(WT / "analysis/cbcimpl_20261029"))
from probs import PROBS  # noqa: E402
MP = sorted({x for part in json.loads((WT / "analysis/cbcimpl_20261029/mitt_parts.json").read_text()) for x in part})
probs = [q for a in sys.argv[3:] for q in (MP if a == "M" else list(PROBS) if a == "P" else [a])]
out = D / f"scanrrc_t{th}.jsonl"
done = {json.loads(l)["problem"] for l in out.read_text().splitlines()} if out.exists() else set()
pat = re.compile(r"root reduced cost: (\d+) lurking bounds, fixing rate ([\d.]+), free columns ([\d.]+)")
for p in probs:
    if p in done: continue
    try: mps = rb.fetch(p, WT / ".miplib_cache")
    except Exception as e:  # noqa: BLE001
        rec = {"problem": p, "error": str(e)[:100]}
    else:
        env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
        env.update({"ENOMOTO_MIP_THREADS": th, "ENOMOTO_T_MIP_SEED": "1", "ENOMOTO_MIP_LOG": "1"})
        try:
            r = subprocess.run([sys.executable, str(WT / "scripts/run_miplib_benchmark.py"), "--worker-ours", str(mps), "--time-limit", tl, "--threads", th],
                               cwd=WT, env=env, capture_output=True, text=True, timeout=float(tl) * 3 + 60)
            err = r.stderr
        except subprocess.TimeoutExpired as e:
            err = (e.stderr or b"").decode() if isinstance(e.stderr, bytes) else (e.stderr or "")
        m = pat.search(err)
        rec = {"problem": p}
        if m:
            nl, fr, fc = m.groups()
            rec.update({"lurking": int(nl), "rate": float(fr), "free": float(fc)})
        else:
            rec["probe"] = None
    with open(out, "a") as f: f.write(json.dumps(rec) + "\n")
    print(rec, flush=True)
