"""ENOMOTO のログ (ENOMOTO_MIP_LOG=1) から最初の根の情報を JSON で出す:
前景のカットのラウンド数・その後の LP の値・裏のカットのラウンド数と値・木に入った時刻 (秒)・止まった理由の行。
tree_at はログの時刻 (再スタートの後はその段の始めからの秒)、tree_abs は再スタートの時刻を足した求解の始めからの秒。
fg_rounds_all は木に入るまでの前景のカットのラウンドの数 (再スタートの後の根を含む。裏のカットの写しのラウンドは含まない)。"""
import json, re, sys
d = {"restarts": 0, "t_off": 0.0, "fg_rounds_all": 0, "fg_rounds": None, "fg_obj": None, "bg_rounds": None, "bg_obj": None, "tree_at": None, "stop": [], "ext": 0}
maxr = -1
for l in open(sys.argv[1], errors="replace"):
    if d["tree_at"] is None:
        m = re.match(r"MIP: restart (\d+) \(.*\) \(([0-9.]+)s\)$", l.strip())
        if m: d["restarts"] += 1; d["t_off"] += float(m.group(2))
        if re.match(r"MIP: cut round \d+: \d+ cuts", l): d["fg_rounds_all"] += 1
        m = re.match(r"MIP: cut round (\d+):", l)
        if m and d["fg_obj"] is None: maxr = max(maxr, int(m.group(1)))
        m = re.search(r"cut loop done: .* -> ([-0-9.e+]+)", l)
        if m and d["fg_obj"] is None: d["fg_obj"] = float(m.group(1)); d["fg_rounds"] = maxr + 1
        m = re.search(r"MIP: background cuts: (\d+) rounds, .* -> ([-0-9.e+]+)", l)
        if m and d["bg_obj"] is None: d["bg_rounds"] = int(m.group(1)); d["bg_obj"] = float(m.group(2))
        if "cut loop extended" in l: d["ext"] += 1
        if re.search(r"cut loop: .*stop|phase 2|-> stop", l) and len(d["stop"]) < 4: d["stop"].append(l.strip()[:160])
    m = re.search(r"MIP: tree search with \d+ threads \(([0-9.]+)s\)", l)
    if m and d["tree_at"] is None: d["tree_at"] = float(m.group(1)); d["tree_abs"] = round(d["t_off"] + d["tree_at"], 2)
print(json.dumps(d))
