"""(設定の値 UNSET はその変数を外す。CMP_OUT で出力のファイル、CMP_NO_HIGHS で HiGHS を解かない)
書き出したノードの LP (dumps/<問題>/index.txt) を、同じ出発の基底から ENOMOTO (cut_lp_debug テスト) と HiGHS (highs_warm.py) で解き直して並べる。
使い方: python cmp_dumps.py <問題> [ENOMOTO の設定 名前=K=V;K=V ...]   出力: cmp_<問題>.jsonl と表
ENOMOTO の既定の回は「重みを引き継ぐ」(CUTLP_DSE=1 ENOMOTO_WARM_DSE=1、木の中と同じ)。TEST_BIN でテストの実行ファイル。"""
import json, os, re, subprocess, sys
from pathlib import Path
D = Path(__file__).resolve().parent
TB = os.environ.get("TEST_BIN", "/tmp/tgt/release/deps/_core-f6bdaa5b0a0ae24b")
p = sys.argv[1]
cfgs = {"eno": {"CUTLP_DSE": "1", "ENOMOTO_WARM_DSE": "1"}}
for a in sys.argv[2:]:
    name, kv = a.split("=", 1)
    c = dict(cfgs["eno"], **dict(x.split("=", 1) for x in kv.split(";") if x))
    cfgs[name] = {k: v for k, v in c.items() if v != "UNSET"}
idx = [l.split() for l in open(D / "dumps" / p / "index.txt")]
out = open(D / os.environ.get("CMP_OUT", f"cmp_{p}.jsonl"), "a")
no_highs = os.environ.get("CMP_NO_HIGHS") is not None
for t in idx:
    prefix = t[0]
    rec = dict(zip(t[1::2], t[2::2]))
    row = {"prefix": Path(prefix).name, "kind": rec["kind"], "depth": rec["depth"], "rows": rec["rows"], "tree_iters": int(rec["iters"]), "tree_status": rec["status"],
           "prim_inf": int(re.findall(r"rec \[(\d+), (\d+)", " ".join(t))[0][1])}
    for name, env in cfgs.items():
        e = dict(os.environ, CUTLP=prefix, LD_LIBRARY_PATH="/usr/local/lib", CUTLP_TIME="60", **env)
        r = subprocess.run([TB, "cut_lp_debug", "--ignored", "--nocapture"], env=e, capture_output=True, text=True)
        m = re.search(r"CUTLP: rep=0 status=(\w+) iters=(\d+) time=([\d.]+)s", r.stderr)
        row[name] = (int(m.group(2)), float(m.group(3)), m.group(1)) if m else None
    r = subprocess.run([sys.executable, "-I", str(D / "highs_warm.py"), prefix] if not no_highs else ["true"], capture_output=True, text=True)
    try:
        h = json.loads(r.stdout.strip().splitlines()[-1])
        row["highs"] = (h["iters"], h["secs"], h["status"])
    except Exception:  # noqa: BLE001
        row["highs"] = None
    out.write(json.dumps(row) + "\n"); out.flush()
    print(row["prefix"], "kind", row["kind"], "inf", row["prim_inf"], "tree", row["tree_iters"], {k: (v[0], v[1]) if v else None for k, v in row.items() if k in cfgs or k == "highs"}, flush=True)
