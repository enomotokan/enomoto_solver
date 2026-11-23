"""書き出したノードの LP (dumps/<問題>/index.txt) を、同じ出発の基底から当方 (cut_lp_debug テスト、ENOMOTO_PROF_PHASES_EXT つき、
重みを引き継ぐ) と HiGHS (analysis/node_lp_20261106/highs_warm.py) で解き直し、反復数・時間・当方の段階別の時間を res_<問題>.jsonl に書く。
使い方 (リポジトリの根から): python analysis/lpiter_20261123/cmp.py <問題> [名札] [K=V;K=V]   (名札・追加の環境変数は当方の回だけ)
TEST_BIN でテストの実行ファイル (既定 target/test-py/release/deps/_core-* の最新)。CMP_NO_HIGHS で HiGHS を解かない。"""
import glob, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
tb = os.environ.get("TEST_BIN") or max(glob.glob(str(R / "target/test-py/release/deps/_core-*")), key=os.path.getmtime)
tb = [t for t in [tb] if not t.endswith(".d")][0]
p = sys.argv[1]
tag = sys.argv[2] if len(sys.argv) > 2 else "def"
extra = dict(kv.split("=", 1) for kv in (sys.argv[3] if len(sys.argv) > 3 else "").split(";") if kv)
out = open(D / f"res_{p}.jsonl", "a")
for line in open(D / "dumps" / p / "index.txt"):
    t = line.split()
    prefix = t[0]
    rec = dict(zip(t[1::2], t[2::2]))
    row = {"tag": tag, "extra": extra, "prefix": Path(prefix).name, "depth": int(rec["depth"]), "rows": int(rec["rows"]), "tree_iters": int(rec["iters"]),
           "tree_secs": float(rec["secs"])}
    e = dict(os.environ, CUTLP=prefix, LD_LIBRARY_PATH="/usr/local/lib", CUTLP_TIME="60", CUTLP_DSE="1", ENOMOTO_WARM_DSE="1", ENOMOTO_PROF_PHASES_EXT="1", **extra)
    r = subprocess.run([tb, "cut_lp_debug", "--ignored", "--nocapture"], env=e, capture_output=True, text=True)
    m = re.search(r"CUTLP: rep=0 status=(\w+) iters=(\d+) time=([\d.]+)s", r.stderr)
    row["ours"] = (m.group(1), int(m.group(2)), float(m.group(3))) if m else None
    ph = {}
    for l in r.stderr.splitlines():
        mm = re.match(r"\s+([a-z_()]+)\s+([\d.]+)ms\s+[\d.]+% of wall", l)
        if mm:
            ph[mm.group(1)] = ph.get(mm.group(1), 0.0) + float(mm.group(2))
        mm = re.match(r"PROF_PHASES_EXT wall=([\d.]+)ms iters=(\d+) .*refactor_count=(\d+)", l)
        if mm:
            ph["wall"] = ph.get("wall", 0.0) + float(mm.group(1)); ph["iters"] = ph.get("iters", 0) + int(mm.group(2)); ph["refactors"] = ph.get("refactors", 0) + int(mm.group(3))
        mm = re.search(r"final_expected_density col_aq=([\d.]+) bfrt=([\d.]+)", l)
        if mm:
            ph["dens_col_aq"] = float(mm.group(1))
        mm = re.search(r"work/iter \(m=(\d+)\): rho_nnz=([\d.]+) price_entries=([\d.]+).*alpha_nnz=([\d.]+) tau_nnz=([\d.]+)", l)
        if mm:
            ph["rho_nnz"] = float(mm.group(2)); ph["alpha_nnz"] = float(mm.group(4)); ph["tau_nnz"] = float(mm.group(5))
        mm = re.search(r"avg_infeasible_pool/iter=([\d.]+)", l)
        if mm:
            ph["inf_pool"] = float(mm.group(1))
    row["phases"] = ph
    if os.environ.get("CMP_NO_HIGHS") is None:
        h = subprocess.run([sys.executable, "-I", str(R / "analysis/node_lp_20261106/highs_warm.py"), prefix], capture_output=True, text=True)
        try:
            hj = json.loads(h.stdout.strip().splitlines()[-1])
            row["highs"] = (hj["status"], hj["iters"], hj["secs"])
        except Exception:
            row["highs"] = None
    out.write(json.dumps(row) + "\n"); out.flush()
    o, hh = row["ours"], row.get("highs")
    print(row["prefix"], row["rows"], "ours", o, "%.1fus" % (1e6 * o[2] / max(o[1], 1)) if o else "", "highs", hh, "%.1fus" % (1e6 * hh[2] / max(hh[1], 1)) if hh else "", flush=True)
