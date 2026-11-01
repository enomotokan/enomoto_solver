"""logs/ の各回から根のカットのループの道と結果を表にする。使い方: python analyze.py [問題 [設定,...]]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
prob = sys.argv[1] if len(sys.argv) > 1 else "neos-960392"
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
res = {}
for line in (D / "results.jsonl").read_text().splitlines():
    r = json.loads(line)
    res[(r["prob"], r["cfg"], r["rep"])] = r
rows = []
for (p, c, k), r in sorted(res.items(), key=lambda t: (t[0][1], t[0][2])):
    if p != prob or (cfgs and c not in cfgs):
        continue
    t = (D / "logs" / f"{p}_{c}_{k}.log").read_text(errors="replace")
    rounds = re.findall(r"MIP: cut round (\d+): (\d+) cuts .*?LP (\d+) iters\).*?obj (\S+) \(([\d.]+)s\)", t)
    fails = re.findall(r"MIP: cut round (\d+): LP status (\w+), removed", t)
    stop = ""
    if "ENOMOTO_T_MIP_CUT_FLAT_STOP" in t: stop = "flat"
    elif "ENOMOTO_MIP_CUT_LP_CAP_STOP" in t: stop = "lpcap"
    elif re.search(r"work cap check: .* -> stop", t): stop = "work"
    elif re.search(r"time cap check: .* -> stop", t): stop = "time"
    elif "integral objective, LP bound unchanged" in t: stop = "intobj"
    m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP.*obj (\S+) -> (\S+)", t)
    done = f"{m.group(1)}/{m.group(2)}" if m else "-"
    lb = f"{float(m.group(4)):.6g}" if m else "-"
    # ループの終わりの時刻 (前処理の後から): 最後のラウンドの行の時刻か、fail の後
    tl = [float(x[4]) for x in rounds]
    # ループの終わりの時刻 (前処理の後から): 「cut loop done」の後で最初に時刻の出る行。カットのループに入った時刻は選択の比べの行
    mr = re.search(r"cut loop done.*?\n(?:.*\n)*?.*?([\d.]+)s\)", t)
    mp = re.search(r"cut selection probe \(\d+ rounds\).*, ([\d.]+)s\)\n", t)
    # clique の fallback の判定 (ループの後、neos-960392 では約 6 秒) の分は引く
    mf = re.search(r"clique fallback: .*\(([\d.]+)s\)\nMIP: cut loop done", t)
    tend = (f"{float(mp.group(1)):.1f}→" if mp else "") + (f"{float(mr.group(1)) - (float(mf.group(1)) if mf else 0):.1f}" if mr else "-")
    rens = re.search(r"parallel sub-MIP started \(RENS\): (\d+) free", t)
    rr = re.search(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+), objective (?:Some\()?([-\d.e]+|None)\)?, ([\d.]+)s", t)
    checks = re.findall(r"cut round (\d+): root cut time cap check: clock ([\d.]+)s of cap ([\d.]+)s", t)
    ch = " ".join(f"r{a}:{float(b):.2f}/{float(cc):.2f}" for a, b, cc in checks)
    rl = " ".join(f"{a}:{b}c/{d}it" for a, b, d, _, _ in rounds) + "".join(f" {a}:{s}" for a, s in fails)
    rows.append((c, k, r.get("status"), r.get("objective"), round(r.get("time") or 0, 1), r.get("nodes"), rl, stop, done, lb, tend, rens.group(1) if rens else "-",
                 f"{rr.group(2)}n {rr.group(3)} {rr.group(4)}s" if rr else "-", ch))
print("| 設定 | 回 | 結果 | 暫定解 | 時間 | ノード | ラウンド (本数/反復、外した LP) | 止めた理由 | 足した/LP | 根の下界 | ループの終わり (秒) | RENS の自由な列 | RENS | 時間の判定 (時計/上限) |")
print("|" + "---|" * 14)
for row in rows:
    print("| " + " | ".join(str(x) for x in row) + " |")
