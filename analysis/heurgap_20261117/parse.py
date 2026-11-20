"""調べ C のログの集計: python parse.py > parse.txt
- HiGHS: 進み具合の表の行のうち、頭に手法の記号 (L = サブ MIP、C = 中心丸め、J = 実行可能性ジャンプ、B = 分枝、T = 節点の評価、R・F・H など) が
  付いた行 (= 暫定解の更新) と、再スタートの行を、時刻・下界・暫定解と並べる。
- 当方: [起動からの秒] 付きのログから、暫定解の更新 (new incumbent … [手法])・再スタート・根のカットのループの終わり・木の始まりと、
  サブ MIP ごとの結果 (手法、状態、節点、秒) を拾い、手法ごとの回数・当たり (解を返した)・秒の合計を出す。"""
import re, glob, collections
from pathlib import Path
D = Path(__file__).resolve().parent / "logs"
HROW = re.compile(r"^ ([A-Za-z]) +(\d+) +\d+ +\d+ +[\d.]+% +(\S+) +(\S+) +\S+ .* ([\d.]+)s$")
for f in sorted(glob.glob(str(D / "highs_*.log"))):
    print(f"\n== {Path(f).name}")
    for l in open(f, errors="replace"):
        m = HROW.match(l.rstrip())
        if m: print(f"  {float(m.group(5)):7.1f}s  [{m.group(1)}] 暫定解 {m.group(4)}  下界 {m.group(3)}  節点 {m.group(2)}")
        elif "restarting" in l or l.startswith("Model after restart"): print("          ", l.strip())
TS = re.compile(r"^\[ *([\d.]+)\] (.*)$")
for f in sorted(glob.glob(str(D / "ours_*.log"))):
    print(f"\n== {Path(f).name}")
    stat = collections.defaultdict(lambda: [0, 0, 0.0])
    for l in open(f, errors="replace"):
        m = TS.match(l.rstrip())
        if not m: continue
        t, s = float(m.group(1)), m.group(2)
        if "new incumbent" in s:
            mm = re.search(r"new incumbent (\S+) at node (\d+) .*\[(.*)\]", s)
            print(f"  {t:7.1f}s  [{mm.group(3)}] 暫定解 {float(mm.group(1)):.6g}  節点 {mm.group(2)}")
        elif re.search(r"MIP: restart \d+ \(|early restart:|cut loop done|tree search with|after root heuristics", s):
            print(f"  {t:7.1f}s  {s[5:150]}")
        mm = re.search(r"parallel sub-MIP \((.+?)\): status (\w+), nodes (\d+), objective (\S+?), ([\d.]+)s", s)
        if mm:
            e = stat["root " + mm.group(1)]; e[0] += 1; e[1] += mm.group(4) != "None"; e[2] += float(mm.group(5))
        mm = re.search(r"SUBFIX (\w+ \w+) try \d+: .*status Some\((\w+)\), nodes (\d+), accepted (\w+), ([\d.]+)s", s)
        if mm:
            e = stat["SUBFIX " + mm.group(1)]; e[0] += 1; e[1] += mm.group(4) == "true"; e[2] += float(mm.group(5))
        mm = re.search(r"root heuristic (.+?): (none|found|skipped|improved)\S* ?\(?([\d.]+)?", s)
        if mm and mm.group(2) != "skipped":
            e = stat["root heur " + mm.group(1)]; e[0] += 1; e[1] += mm.group(2) != "none"; e[2] += float(mm.group(3) or 0)
    print("  手法ごと (回数・解を返した回数・秒の合計):")
    for k, (n, h, sec) in sorted(stat.items(), key=lambda kv: -kv[1][2]):
        print(f"    {k:40s} {n:3d} {h:3d} {sec:8.2f}s")
