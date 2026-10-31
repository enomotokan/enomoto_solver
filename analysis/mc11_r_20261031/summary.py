#!/usr/bin/env python3
"""logs/*.log と *.json から、区間 (再スタートごと) の根の様子を表にする。
区間ごとに: 根のカットのループの終わり (cut loop done の時刻 = 根のヒューリスティクスの始め)、根のヒューリスティクスの終わり、
bgw の待ち (秒・上限・理由)、裏のカット (ラウンド・目的値)、置き換え後の下界、区間の終わりの時刻とノード数。
使い方: python3 summary.py [名前の接頭辞 ...]"""
import json, re, sys
from pathlib import Path

d = Path(__file__).parent / "logs"
pre = sys.argv[1:]
T = r"\(([\d.]+)s\)"


def segs(log):
    # 再スタートの行で区切る ("MIP: restart N (" で始まる行)
    out, cur = [], []
    for ln in log.splitlines():
        if re.match(r"MIP: restart \d+ \(", ln):
            out.append((cur, ln))
            cur = []
        else:
            cur.append(ln)
    out.append((cur, None))
    return out


def seg_info(lines, rline):
    s = "\n".join(lines)
    o = {}
    # 区間の始めの時刻 (最後のカットのラウンドの時刻で根のカットのループの終わりを測る)
    cr = re.findall(r"^MIP: cut round \d+: .* \(([\d.]+)s\)$", s, re.M)
    o["cut_end"] = float(cr[-1]) if cr else None
    m = re.search(r"after root heuristics: incumbent (?:Some\(([\d.e+-]+)\)|None) " + T, s)
    o["heur_end"] = float(m.group(2)) if m else None
    o["inc"] = m.group(1) if m else None
    m = re.search(r"background cuts: waited ([\d.]+)s for progress \(cap ([\d.]+)s, (\w+)\), (\d+) rounds, obj (?:Some\(([\d.e+-]+)\)|None)", s)
    o["wait"] = (float(m.group(1)), float(m.group(2)), m.group(3), int(m.group(4)), m.group(5)) if m else None
    m = re.search(r"background cuts: not waiting \(remaining ([\d.]+)s, cap ([\d.]+)s", s)
    o["nowait"] = m.group(0) if m else None
    m = re.search(r"background cuts: (\d+) rounds, \d+ cut rows \(was \d+\), obj ([\d.e+-]+) -> ([\d.e+-]+) \(waited ([\d.]+)s, ([\d.]+)s\)", s)
    if m:
        o["bg"] = (int(m.group(1)), float(m.group(3)), float(m.group(5)), "used")
    else:
        m = re.search(r"background cuts: (\d+) rounds, obj ([\d.e+-]+) not above ([\d.e+-]+)", s)
        if m:
            o["bg"] = (int(m.group(1)), float(m.group(2)), None, "not above %.6g" % float(m.group(3)))
        elif "background cuts: no result" in s:
            o["bg"] = (0, None, None, "no result")
        else:
            o["bg"] = None
    nl = re.findall(r"^MIP:\s+([\d.]+)s nodes\s+(\d+) open\s+\d+ lb (\S+) ub", s, re.M)
    o["nodes"] = (float(nl[-1][0]), int(nl[-1][1]), nl[-1][2]) if nl else None
    o["restart"] = None
    if rline:
        m = re.search(r"restart (\d+) \((\d+) of (\d+) integer columns newly fixed.*\(([\d.]+)s\)", rline)
        o["restart"] = (int(m.group(2)), float(m.group(4))) if m else rline
    return o


def fmt(x, f="%.2f"):
    return "-" if x is None else f % x


print("| 回 | 結果 | 時間 | 区間 | カットの終わり | ヒューリスティクスの終わり (長さ) | 待ち (上限・理由) | 裏のカット (ラウンド・目的値) | 置き換え後 | 区間の終わり (ノード・lb) |")
print("|---|---|---|---|---|---|---|---|---|---|")
for js in sorted(d.glob("*.json")):
    name = js.stem
    if pre and not any(name.startswith(p) for p in pre):
        continue
    try:
        r = json.loads(js.read_text())
    except Exception:
        continue
    log = (d / f"{name}.log").read_text()
    st = r["status"] + (f" {r['objective']:.6g}" if r["status"] != "optimal" and r.get("objective") is not None else "")
    for k, (lines, rl) in enumerate(segs(log)):
        o = seg_info(lines, rl)
        hl = (o["heur_end"] - o["cut_end"]) if o["heur_end"] is not None and o["cut_end"] is not None else None
        heur = f"{fmt(o['heur_end'])} ({fmt(hl)})"
        if o["wait"]:
            w = o["wait"]
            wt = f"{w[0]:.2f} ({w[1]:.2f}・{w[2]})"
        elif o["nowait"]:
            wt = "待たず"
        else:
            wt = "-"
        bg = o["bg"]
        bgs = "-" if bg is None else (f"{bg[0]}・{bg[1]:.1f}" if bg[1] is not None else f"{bg[0]}・-") + ("" if bg[3] == "used" else f" ({bg[3]})")
        rep = "-" if bg is None or bg[2] is None else f"{bg[2]:.2f}"
        if o["restart"] and isinstance(o["restart"], tuple):
            end = f"再スタート {o['restart'][1]:.2f} 秒 (固定 {o['restart'][0]})"
        elif o["nodes"]:
            end = f"{o['nodes'][0]:.1f} 秒 ({o['nodes'][1]}・{o['nodes'][2]})"
        else:
            end = "-"
        print(f"| {name if k == 0 else ''} | {st if k == 0 else ''} | {r['time']:.1f} | {k} | {fmt(o['cut_end'])} | {heur} | {wt} | {bgs} | {rep} | {end} |" if k == 0 else
              f"| | | | {k} | {fmt(o['cut_end'])} | {heur} | {wt} | {bgs} | {rep} | {end} |")
