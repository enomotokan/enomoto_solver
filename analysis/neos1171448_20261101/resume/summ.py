"""neos-1171448 の resume・d2lp1 のログの集計 (2026-11-01)。
使い方: python summ.py [ログのディレクトリ (既定 logs)]
1 回ごとに: 状態・時間・主の値・最後の下界・ノード・再スタートの回数 (early restart・restart の行)・木に入る時刻 (`tree search with`)・
根の RENS の終わりの時刻と結果・RENS のやり直しが走った (`RENS (more nodes)` のサブ MIP)/省かれた (`skipped`)/継続した (`resuming`) 回数・
最後の暫定解の時刻と出どころ。どれも外側 (ENOMOTO_MIP_LOG=1 だけなのでサブ MIP の中の行はない) の行。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
L = D / (sys.argv[1] if len(sys.argv) > 1 else "logs")
INC = re.compile(r"MIP: new incumbent (\S+) at node (\d+) \((\S+)s\) \[(.*?)\]")
TREE = re.compile(r"MIP: tree search with \d+ threads \((\S+)s\)")
RHE = re.compile(r"MIP: after root heuristics: incumbent \S+ \((\S+)s\)")
for c in ("resume", "d2lp1", "def"):
    if not (L / c).is_dir():
        continue
    for f in sorted((L / c).glob("*.log"), key=lambda p: int(p.stem.rsplit("_r", 1)[1])):
        txt = f.read_text(errors="replace")
        res = next((json.loads(l) for l in txt.splitlines() if l.startswith("{") and '"status"' in l), {})
        tree = [float(m) for m in TREE.findall(txt)]
        rhe = [float(m) for m in RHE.findall(txt)]
        incs = INC.findall(txt)
        restarts = len(re.findall(r"MIP: early restart:|MIP: restart \d+:", txt))
        restart_lines = len(re.findall(r"^MIP: root LP after the restart", txt, re.M))
        ran = len(re.findall(r"sub-MIP \(RENS \(more nodes\)\)|sub-MIP started \(RENS \(more nodes\)\)|root heuristic RENS \(more nodes\)", txt))
        skipped = len(re.findall(r"RENS \(more nodes\) skipped", txt))
        resumed = len(re.findall(r"RENS \(more nodes\): resuming", txt))
        dropped = len(re.findall(r"RENS \(more nodes\): paused sub-MIP .* dropped", txt))
        rens = re.findall(r"parallel sub-MIP \(RENS\): status (\w+), nodes (\d+), objective (\S+), (\S+)s|MIP:   sub-MIP: (\d+) free columns of \d+, status (\w+), nodes (\d+)", txt)
        last = incs[-1] if incs else None
        print(f"{c:6} {f.stem.rsplit('_',1)[1]:>3} {res.get('status','?'):10} {res.get('time',0):5.1f}s obj {res.get('objective')} bound {res.get('bound')} nodes {res.get('nodes')} "
              f"restarts {restarts} (root LP after restart {restart_lines}) after-root-heur {rhe} tree {tree} "
              f"RENS-more ran {ran} skipped {skipped} resumed {resumed} dropped {dropped} "
              f"last-inc {last[0] + ' @' + last[2] + 's [' + last[3] + ']' if last else None}")
