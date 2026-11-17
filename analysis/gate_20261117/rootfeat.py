"""GATE: ログ (slow2 の one.sh の形、行頭に経過秒) から外側の最初の根の特徴を拾う。python rootfeat.py <ログ> ...
z0 = 根の LP、zc = 最初の根のカットの最後のラウンドの目的値、rounds、根の LP の大きさ、根の段の秒 (カットの終わりまで)、結果の目的値。"""
import re, sys, json, gzip
def feat(fn):
    op = gzip.open if fn.endswith(".gz") else open
    L = op(fn, "rt", errors="replace").read().splitlines()
    f = {"log": fn.split("/")[-1]}
    for l in L:
        m = re.match(r"\[\s*([\d.]+)\] (.*)", l)
        if not m: continue
        t, s = float(m.group(1)), m.group(2)
        if "z0" not in f and (mm := re.match(r"MIP: root LP ([-\d.e+]+) \((\d+) iters, ([\d.]+)s\)", s)):
            f["z0"] = float(mm.group(1)); f["t_lp"] = t; f["lp_secs"] = float(mm.group(3)); f["lp_iters"] = int(mm.group(2))
        if "rows" not in f and (mm := re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", s)):
            f["rows"], f["cols"], f["nnz"] = map(int, mm.groups())
        if (mm := re.match(r"CUTMIX types .* m (\d+) n (\d+) .*introw ([\d.]+)", s)) and "introw" not in f:
            f["introw"] = float(mm.group(3))
        if "z0" in f and "cutend" not in f and (mm := re.match(r"MIP: cut round (\d+): (\d+) cuts .*LP rows (\d+).*obj ([-\d.e+]+)", s)):
            f["rounds"] = int(mm.group(1)) + 1; f["zc"] = float(mm.group(4)); f["t_cut"] = t; f["ncuts"] = f.get("ncuts", 0) + int(mm.group(2)); f["lprows_end"] = int(mm.group(3))
        if "z0" in f and "rounds" in f and "cutend" not in f and ("SUBWAIT" in s or "sub-MIP" in s or "branching" in s.lower() or "restart" in s.lower()):
            f["cutend"] = t
        if "SUBWAIT" in s: f.setdefault("subwait", []).append(s[:160])
        if s.startswith("{"):
            try: r = json.loads(s); f["obj"] = r.get("objective"); f["time"] = r.get("time"); f["nodes"] = r.get("nodes"); f["status"] = r.get("status")
            except Exception: pass
    if "z0" in f and "zc" in f and f.get("obj") is not None:
        d = f["obj"] - f["z0"]
        f["closed"] = (f["zc"] - f["z0"]) / d if abs(d) > 1e-9 * max(1, abs(f["obj"])) else 1.0
        f["rgap"] = abs(f["obj"] - f["zc"]) / max(1, abs(f["obj"]))
    return f
for fn in sys.argv[1:]:
    f = feat(fn)
    print(json.dumps({k: (round(v, 4) if isinstance(v, float) else v) for k, v in f.items() if k != "subwait"}, ensure_ascii=False))
