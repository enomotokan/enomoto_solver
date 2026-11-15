"""BINHEUR の計測: 問題 x 設定 x 種を 4 スレッドで 1 つずつ解き、jsonl に足す (途中から再開できる。設定の順は種ごとに逆)。
種は ENOMOTO_T_MIP_SEED=種。計測する wheel は環境変数 BH_PKG (pip install --target の場所) を PYTHONPATH の先に置いて固定する。
どの設定も ENOMOTO_MIP_LOG=1 で解き、暫定解の更新 (値・時刻・出どころ) を "incs" に残す。
使い方: python run_b.py <出力.jsonl> <設定,...> <種,...> <時間> <問題 ...|@58|@hit>"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
WT = D.parents[1]
sys.path.insert(0, str(WT / "analysis" / "cbcimpl_20261029"))
sys.path.insert(0, str(WT / "scripts"))
from probs import PROBS as P58  # noqa: E402
import run_miplib_benchmark as rb  # noqa: E402
CFGS = {
    "def": {},
    "swap": {"ENOMOTO_MIP_LMIP_SWAP": "1"},
    # 組み合わせ (RESTCHAIN の ebiR は Local-MIP を根の後も木に入る直前まで延ばす。ROLE_LMIP は木のヒューリスティクス役で回す)
    "ebiR": {"ENOMOTO_MIP_PRECHAIN": "ebiR"},
    "ebiRs": {"ENOMOTO_MIP_PRECHAIN": "ebiR", "ENOMOTO_MIP_LMIP_SWAP": "1"},
    "rl": {"ENOMOTO_MIP_ROLE_LMIP": "1"},
    "rls": {"ENOMOTO_MIP_ROLE_LMIP": "1", "ENOMOTO_MIP_LMIP_SWAP": "1"},
}
# 集合分割の行があって、解を見つける側が遅かった問題 (summ_t60.txt・summ_t600.txt) と air03
SETS = {"@58": P58, "@hit": ["eil33-2", "nw04", "air03", "bppc4-08", "neos-1456979", "neos-911970", "30n20b8", "fiball", "satellites2-60-fs"]}
out, cfgs, seeds, tl = Path(sys.argv[1]), sys.argv[2].split(","), [int(s) for s in sys.argv[3].split(",")], int(sys.argv[4])
probs = []
for a in sys.argv[5:]:
    probs += SETS.get(a, [a])
if not out.is_absolute():
    out = D / out
done = set()
if out.exists():
    for l in out.read_text().splitlines():
        if l.strip():
            r = json.loads(l); done.add((r["problem"], r["cfg"], r["seed"]))
RE = re.compile(r"MIP: new incumbent (\S+) .*\(([0-9.]+)s\)(?: \[(.*)\])?")
for seed in seeds:
    cs = cfgs if seed % 2 else cfgs[::-1]
    for p in probs:
        for c in cs:
            if (p, c, seed) in done:
                continue
            env = {k: v for k, v in os.environ.items() if not k.startswith("ENOMOTO_")}
            if os.environ.get("BH_PKG"):
                env["PYTHONPATH"] = os.environ["BH_PKG"] + os.pathsep + env.get("PYTHONPATH", "")
            env.update(ENOMOTO_MIP_THREADS="4", ENOMOTO_T_MIP_SEED=str(seed), ENOMOTO_MIP_LOG="1", **CFGS[c])
            mps = rb.fetch(p, WT / ".miplib_cache")
            incs, warn = [], []
            try:
                r = subprocess.run([sys.executable, str(WT / "scripts" / "run_miplib_benchmark.py"), "--worker-ours", str(mps),
                                    "--time-limit", str(tl), "--threads", "4"], cwd=WT, env=env, capture_output=True, text=True, timeout=tl + 400)
                res = json.loads([l for l in r.stdout.splitlines() if l.startswith("{")][-1])
                incs = [(float(m.group(1)), float(m.group(2)), m.group(3)) for m in map(RE.search, r.stderr.splitlines()) if m]
                warn = [l for l in r.stderr.splitlines() if "MIP: WARNING" in l][:5]
            except Exception as e:  # noqa: BLE001
                res = {"status": "crash", "error": str(e)[:300]}
            row = {"problem": p, "cfg": c, "seed": seed, "tl": tl, "result": res, "incs": incs, "warn": warn}
            with open(out, "a") as f:
                f.write(json.dumps(row) + "\n")
            print(p, c, seed, res.get("status"), res.get("time"), res.get("objective"), flush=True)
