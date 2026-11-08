"""ログ (logs/<OUT>_<問題>_<設定>_s<種>.txt) の SEPLEARN の行から、分離器ごとの伸びの割合・採った本数・生き残り・時間の表を作る。
使い方: python summ.py OUT 設定  (根と木 (全スレッドの和) を種で平均)"""
import re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
OUT, CFG = sys.argv[1], sys.argv[2]
pat = re.compile(r"SEPLEARN (root restarts \d+|tree t\d+) (root|node) (\S+)\s+calls (\d+) cands (\d+) chosen (\d+) gain (\S+) \(\S+\) surv (\d+)/(\d+) \(\S+\) secs (\S+) work \S+ skipped (\d+) capped (\d+)")
rows = defaultdict(lambda: defaultdict(lambda: [0.0] * 8))  # (問題, 根/木) -> 分離器 -> [伸び, 採った, 生残, 生残の母数, 秒, 飛ばした, 候補, 回]
nrep = defaultdict(set)
for f in sorted(D.glob(f"logs/{OUT}_*_{CFG}_s*.txt")):
    m = re.match(rf"{OUT}_(.+)_{CFG}_s(\d+)\.txt", f.name)
    prob, s = m.group(1), m.group(2)
    nrep[prob].add(s)
    for l in f.read_text().splitlines():
        mm = pat.match(l)
        if not mm:
            continue
        who, w, src = mm.group(1), mm.group(2), mm.group(3)
        if who.startswith("tree") and w == "root":
            continue  # 主のスレッドが出す根の分は根のループの終わりの行と同じ
        if who.startswith("root") and w == "node":
            continue
        e = rows[(prob, w)][src]
        e[0] += float(mm.group(7)); e[1] += int(mm.group(6)); e[2] += int(mm.group(8)); e[3] += int(mm.group(9))
        e[4] += float(mm.group(10)); e[5] += int(mm.group(11)); e[6] += int(mm.group(5)); e[7] += int(mm.group(4))
SRCS = ["rows", "zerohalf", "clique", "implbd", "path_highs", "path", "tableau", "mcf", "modk", "pool"]
print(f"| 問題 | 根/木 | " + " | ".join(SRCS) + " |")
print("|---|---|" + "---|" * len(SRCS))
for (prob, w) in sorted(rows):
    d = rows[(prob, w)]
    tg = sum(e[0] for e in d.values())
    cells = []
    for s in SRCS:
        e = d.get(s)
        if not e or (e[1] == 0 and e[6] == 0 and e[5] == 0):
            cells.append("")
            continue
        g = 100 * e[0] / tg if tg > 0 else 0
        sv = f"{100 * e[2] / e[3]:.0f}" if e[3] else "-"
        k = len(nrep[prob])
        txt = f"{g:.0f}% / {e[1] / k:.0f} / {sv}% / {e[4] / k:.2f}s"
        if e[5]:
            txt += f" (飛{e[5] / k:.0f})"
        cells.append(txt)
    print(f"| {prob} | {'根' if w == 'root' else '木'} | " + " | ".join(cells) + " |")
print("\nセル: 伸びの割合 / 採った本数 (種の平均) / 生き残り / 分離の時間 (種の平均) (学習で飛ばした呼び出し)")
