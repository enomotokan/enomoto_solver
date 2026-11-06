"""ENOMOTO のログ (ENOMOTO_MIP_LOG=1) から最初の根の情報を JSON で出す:
前景のカットのラウンド数・その後の LP の値・裏のカットのラウンド数と値・木に入った時刻 (秒)・止まった理由の行。"""
import json, re, sys
d = {"fg_rounds": None, "fg_obj": None, "bg_rounds": None, "bg_obj": None, "tree_at": None, "stop": [], "ext": 0}
maxr = -1
for l in open(sys.argv[1], errors="replace"):
    if d["tree_at"] is None:
        m = re.match(r"MIP: cut round (\d+):", l)
        if m and d["fg_obj"] is None: maxr = max(maxr, int(m.group(1)))
        m = re.search(r"cut loop done: .* -> ([-0-9.e+]+)", l)
        if m and d["fg_obj"] is None: d["fg_obj"] = float(m.group(1)); d["fg_rounds"] = maxr + 1
        m = re.search(r"MIP: background cuts: (\d+) rounds, .* -> ([-0-9.e+]+)", l)
        if m and d["bg_obj"] is None: d["bg_rounds"] = int(m.group(1)); d["bg_obj"] = float(m.group(2))
        if "cut loop extended" in l: d["ext"] += 1
        if re.search(r"cut loop: .*stop|phase 2|-> stop", l) and len(d["stop"]) < 4: d["stop"].append(l.strip()[:160])
    m = re.search(r"MIP: tree search with \d+ threads \(([0-9.]+)s\)", l)
    if m and d["tree_at"] is None: d["tree_at"] = float(m.group(1))
print(json.dumps(d))
