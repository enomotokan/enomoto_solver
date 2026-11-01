"""HiGHS のログの要約: python sumh.py logs/h_<名前>_<n> ... (頭を渡す。.log と .trace を読む)
前処理の後の大きさ、根の LP (値・時刻)、下界が 49324 に着いた時刻、暫定解の更新 (時刻・出どころの文字・値)、再スタート、ノード数・結果。"""
import re, sys

for head in sys.argv[1:]:
    log = open(head + ".log").read().splitlines()
    tr = open(head + ".trace").read().splitlines()
    pre = next((l for l in log if "Presolve reductions" in l), "")
    m = re.search(r"rows (\d+)\(-?\d+\); columns (\d+)", pre)
    size = f"{m.group(1)}x{m.group(2)}" if m else "-"
    t_pre = re.match(r"\[\s*([\d.]+)\]", pre).group(1) if pre else "-"
    rootlp = "-"; first = []; restarts = []; t49 = "-"
    for l in log:
        mm = re.match(r"\[\s*([\d.]+)\]\s+([A-Za-z]?)\s+(\d+)\s+(\d+)\s+(\d+)\s+([\d.]+)%\s+(\S+)\s+(\S+)", l)
        if mm:
            t, src, proc, q, lv, _, bb, bs = mm.groups()
            if rootlp == "-" and proc == "0" and bb != "-inf" and float(bb) > 40000:
                rootlp = f"{float(bb):.1f}@{t}"
            if src:
                first.append(f"{src}{bs}@{t}")
        if "estart" in l:
            restarts.append(l.split("]")[0].strip("[ ") + ":" + l.split("]")[1].strip()[:60])
    for l in tr:
        mm = re.match(r"([\d.]+) .*dual=(-?inf|[0-9][0-9.e+-]*)", l)
        if mm and mm.group(2) not in ("-inf", "inf") and float(mm.group(2)) >= 49323.999:
            t49 = mm.group(1); break
    dual = " ".join(f"{float(re.search('dual=([^ ]+)', l).group(1)):.0f}@{l.split()[0]}" for l in tr
                    if re.search("dual=([0-9.]+)", l) and 47000 < float(re.search("dual=([^ ]+)", l).group(1)) < 49325)
    res = next((l for l in log if "Status" in l), "").split()[-1:]
    nodes = next((l.split()[-1] for l in log if re.search(r"\]\s+Nodes\s+\d+", l)), "-")
    tm = next((l.split()[-1] for l in log if re.search(r"\]\s+Timing\s+[\d.]+", l)), "-")
    print(f"{head.split('/')[-1]:14s} pre {size}@{t_pre}s rootLP {rootlp} lb49324@{t49} sols {' '.join(first[:4])} "
          f"restart {len(restarts)} nodes {nodes} {res[0] if res else '-'} {tm}s")
    print(f"{'':14s} dual: {dual}")
    for r in restarts[:3]: print(f"{'':14s} {r}")
