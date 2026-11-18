"""cutlen: one.sh のログから、外側の最初の根のカットのループの特徴を拾う。python feat.py logs/*.log
w_lp = 根の LP の手間 (tick-s、`MIP: root LP work` の行)、rounds = ラウンド数、w_loop = 最後の時間の上限の判定の時計 (ループの始めからの手間)、
r = 1 ラウンドの手間 ÷ 根の LP の手間 (= w_loop / (rounds-1) / w_lp。判定はラウンド 1 から出るので、判定の回数で割る)、
cap = 時間の上限 (tick-s)、z0 = 根の LP、zc = 最後のラウンドの目的値、closed = 閉じた割合 (結果の目的値まで)、t_cut = ループの終わりの経過秒。"""
import re, sys, json, gzip


def feat(fn):
    op = gzip.open if fn.endswith(".gz") else open
    L = op(fn, "rt", errors="replace").read().splitlines()
    f = {"log": fn.split("/")[-1]}
    nchk = 0
    for l in L:
        m = re.match(r"\[\s*([\d.]+)\] (.*)", l)
        if not m:
            continue
        t, s = float(m.group(1)), m.group(2)
        if "z0" not in f and (mm := re.match(r"MIP: root LP ([-\d.e+]+) \((\d+) iters, ([\d.]+)s\)", s)):
            f["z0"] = float(mm.group(1)); f["lp_iters"] = int(mm.group(2))
        if "w_lp" not in f and (mm := re.match(r"MIP: root LP work: ([\d.e+-]+) tick-s \((\d+) iters, ([\d.]+)s\)", s)):
            f["w_lp"] = float(mm.group(1)); f["lp_secs"] = float(mm.group(3))
        if "rows" not in f and (mm := re.match(r"MIP: root LP size: rows (\d+), free columns (\d+), nonzeros (\d+)", s)):
            f["rows"], f["cols"], f["nnz"] = map(int, mm.groups())
        if "z0" in f and not f.get("end"):
            if (mm := re.match(r"MIP: cut round (\d+): root cut time cap check: clock ([\d.]+)s of cap ([\d.]+)s", s)):
                f["w_loop"] = float(mm.group(2)); f["cap"] = float(mm.group(3)); nchk += 1
                if "-> stop" in s:
                    f["stop"] = "cap"
            if (mm := re.match(r"MIP: cut round (\d+): (\d+) cuts .*LP rows (\d+).*obj ([-\d.e+]+)", s)):
                f["rounds"] = int(mm.group(1)) + 1; f["zc"] = float(mm.group(4)); f["t_cut"] = t
            if re.match(r"MIP: cut round \d+: .*(stall|did not move|no cuts|no candidates)", s) and "stop" not in f:
                f["stop"] = s[s.index(":", 5) + 2:][:80]
            if "rounds" in f and ("restart check" in s or "branching" in s.lower() or "sub-MIP" in s):
                f["end"] = True
        if s.startswith("{"):
            try:
                r = json.loads(s); f["obj"] = r.get("objective"); f["time"] = r.get("time"); f["nodes"] = r.get("nodes"); f["status"] = r.get("status")
            except Exception:  # noqa: BLE001
                pass
    f.pop("end", None)
    if nchk and f.get("w_lp"):
        f["r"] = f["w_loop"] / nchk / max(f["w_lp"], 1e-9)
    if "z0" in f and "zc" in f and f.get("obj") is not None:
        d = f["obj"] - f["z0"]
        f["closed"] = (f["zc"] - f["z0"]) / d if abs(d) > 1e-9 * max(1, abs(f["obj"])) else 1.0
    return f


if __name__ == "__main__":
    for fn in sys.argv[1:]:
        print(json.dumps(feat(fn)))
