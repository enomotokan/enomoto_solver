"""根のカットのループで書き出した LP (ENOMOTO_MIP_DEBUG_DUMP_CUT_LP の <dir>/cutlp_r*_*.mps/.bas) を、同じ出発の基底から
当方 (cut_lp_debug テスト、引き継いだ DSE 重みつき、ENOMOTO_PROF_PHASES_EXT) と HiGHS (node_lp_20261106/highs_warm.py、前処理なし 1 スレッド)
で解き直して比べる。analysis/lpiter_20261123/cmp.py の写し。使い方: python cmpcut.py <dir> <出力.jsonl> [K=V;K=V]"""
import glob, json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
R = D.parents[1]
tb = [t for t in sorted(glob.glob(str(R / "target/test-py/release/deps/_core-*")), key=os.path.getmtime) if not t.endswith(".d")][-1]
dd, outf = sys.argv[1], sys.argv[2]
extra = dict(kv.split("=", 1) for kv in (sys.argv[3] if len(sys.argv) > 3 else "").split(";") if kv)
out = open(outf, "a")
for mps in sorted(glob.glob(f"{dd}/cutlp_r*.mps"), key=lambda s: int(re.search(r"_r(\d+)_", s).group(1))):
    prefix = mps[:-4]
    row = {"prefix": Path(prefix).name, "extra": extra}
    e = dict(os.environ, CUTLP=prefix, LD_LIBRARY_PATH="/usr/local/lib", CUTLP_TIME="60", CUTLP_DSE="1", ENOMOTO_WARM_DSE="1", ENOMOTO_PROF_PHASES_EXT="1")
    e.update(extra)
    r = subprocess.run([tb, "cut_lp_debug", "--ignored", "--nocapture"], env=e, capture_output=True, text=True)
    m = re.search(r"CUTLP: rep=0 status=(\w+) iters=(\d+) time=([\d.]+)s", r.stderr)
    row["ours"] = (m.group(1), int(m.group(2)), float(m.group(3))) if m else r.stderr[-300:]
    ph = {}
    for l in r.stderr.splitlines():
        mm = re.match(r"\s+([a-z_()]+)\s+([\d.]+)ms\s+[\d.]+% of wall", l)
        if mm:
            ph[mm.group(1)] = round(ph.get(mm.group(1), 0.0) + float(mm.group(2)), 2)
        mm = re.match(r"PROF_PHASES_EXT wall=([\d.]+)ms iters=(\d+) .*refactor_count=(\d+)", l)
        if mm:
            ph["wall"] = round(ph.get("wall", 0.0) + float(mm.group(1)), 2); ph["iters"] = ph.get("iters", 0) + int(mm.group(2)); ph["refactors"] = ph.get("refactors", 0) + int(mm.group(3))
    row["ours_phases"] = ph
    if not os.environ.get("CMP_NO_HIGHS"):
        h = subprocess.run([sys.executable, "-I", str(R / "analysis/node_lp_20261106/highs_warm.py"), prefix], capture_output=True, text=True)
        try:
            row["highs"] = json.loads(h.stdout.strip().splitlines()[-1])
        except Exception:
            row["highs"] = h.stderr[-300:]
    out.write(json.dumps(row, ensure_ascii=False) + "\n"); out.flush()
    print(row["prefix"], row["ours"], row.get("highs"))
