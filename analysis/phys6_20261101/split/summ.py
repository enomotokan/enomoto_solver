"""logs/physiciansched6-2_*.log から、解ける回と解けない回を分ける表を作る (summ.txt)。
列: 回・結果 (秒)・49324 を見つけた時刻と出どころ・主のカットのループの後の下界・裏のカットの待ち (秒、理由、ラウンド数、各ラウンドの値と時刻)・
置き換えの後の下界・再スタート (固定数)・木に入る時刻・木の下界 (最初 → 最後)・最後の暫定解と下界。
使い方: python summ.py [設定,...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else None
rows = []
for f in sorted(D.glob("logs/physiciansched6-2_*.log"), key=lambda f: (f.stem.rsplit("_", 1)[0], int(f.stem.rsplit("_", 1)[1]))):
    name = f.stem[len("physiciansched6-2_"):]
    cfg, rep = name.rsplit("_", 1)
    if cfgs and cfg not in cfgs:
        continue
    s = f.read_text()
    js = [l for l in s.splitlines() if l.startswith("{")]
    res = json.loads(js[-1]) if js else {}
    st = "最適 %.1f" % res["time"] if res.get("status") == "optimal" else "T"
    m = re.search(r"new incumbent 4\.9324000000e4 at node \d+ \(([\d.]+)s\) \[([^\]]+)\]", s)
    opt = f"{float(m.group(1)):.1f} {m.group(2)}" if m else "-"
    inc = re.findall(r"new incumbent ([\d.e]+) at node \d+ \(([\d.]+)s\) \[([^\]]+)\]", s)
    first = f"{float(inc[0][0]):.0f} {float(inc[0][1]):.1f} {inc[0][2]}" if inc else "-"
    m = re.search(r"cut loop done: .* -> ([\d.e]+)", s)
    main = f"{float(m.group(1)):.0f}" if m else "-"
    m = re.search(r"background cuts: waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\), (\d+) rounds", s)
    wait = f"{m.group(1)}/{m.group(2)} {m.group(3)} {m.group(4)}" if m else ("not waiting" if "background cuts: not waiting" in s else "-")
    m = re.search(r"background cuts: rounds so far \[([^\]]*)\]", s)
    rs = "; ".join(re.sub(r"(\d+)\.\d at ", r"\1@", x) for x in m.group(1).split(", ")) if m and m.group(1) else "-"
    ps = re.search(r"background cuts reached the prune limit .* \(([\d.]+)s\)", s)
    if ps:
        wait = f"heur中に到達 {float(ps.group(1)):.1f}s / " + wait
    m = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj [\d.e]+ -> ([\d.e]+)", s)
    if m:
        rep_ = f"{float(m.group(2)):.0f} ({m.group(1)}r)"
    elif (m := re.search(r"background cuts: (\d+) rounds, obj ([\d.e]+) not above", s)):
        rep_ = f"不採用 {float(m.group(2)):.0f}"
    else:
        rep_ = "-"
    rst = re.findall(r"MIP: restart (\d+) \((\d+) of \d+ integer columns newly fixed", s)
    rst_s = ",".join(f"{b}" for a, b in rst) if rst else "なし"
    m = re.search(r"tree search with \d+ threads \(([\d.]+)s\)", s)
    tree = f"{float(m.group(1)):.1f}" if m else "-"
    lbs = re.findall(r"nodes +\d+ open +\d+ lb ([\d.e]+|-?inf)", s)
    lbs = [x for x in lbs if "inf" not in x]
    tlb = f"{float(lbs[0]):.0f}→{float(lbs[-1]):.0f}" if lbs else "-"
    fin = f"{res.get('objective') or 0:.0f}/{res.get('bound') or 0:.0f}" if st == "T" else ""
    closed = "閉" if "closed the gap to the prune limit" in s else ""
    rows.append((cfg, rep, st, first, opt, main, wait, rs, rep_, closed, rst_s, tree, tlb, fin))
hdr = ("設定", "回", "結果", "最初の解", "49324 (秒 出どころ)", "主カット後", "裏の待ち (秒/上限 理由 ラウンド)", "裏の各ラウンド (値@秒)", "置き換え後", "根閉", "再スタート固定", "木の開始", "木の下界", "最後 (解/下界)")
out = ["| " + " | ".join(hdr) + " |", "|" + "---|" * len(hdr)]
out += ["| " + " | ".join(r) + " |" for r in rows]
txt = "\n".join(out)
(D / ("summ.txt" if not cfgs else f"summ_{'_'.join(cfgs)}.txt")).write_text(txt + "\n")
print(txt)
