"""SUBCAP の集計: res.jsonl とログ (logs/<設定>_<問題>_s<種>.log[.gz]) から、問題ごと・設定ごとに状態・秒・暫定解・下界・根の終わりの時刻
(`after root heuristics` の秒) と根の後の暫定解・根のサブ MIP (外側のノード 1 以下で始めたもの) の本数・ノードの和・秒の和・最長の秒・解を返した本数・
SUBCAP で止めた数 (葉・重さ・待ち) を出す。
使い方: python summ.py [問題,...]"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
want = set(sys.argv[1].split(",")) if len(sys.argv) > 1 else None


def readlog(tag, p, seed):
    for f in (D / "logs" / f"{tag}_{p}_s{seed}.log", D / "logs" / f"{tag}_{p}_s{seed}.log.gz"):
        if f.exists():
            return (gzip.open(f, "rt", errors="replace") if f.suffix == ".gz" else open(f, errors="replace")).read()
    return ""


def g(v):
    return "-" if v is None else f"{v:.6g}"


rows = defaultdict(list)
for l in open(D / "res.jsonl"):
    r = json.loads(l)
    if want and r["problem"] not in want:
        continue
    rows[r["problem"]].append(r)
for p, rs in rows.items():
    print(f"## {p}\n")
    print("| 設定 | 種 | 状態 | 秒 | 暫定解 | 下界 | 根の終わり (秒) | 根の後 | 根のサブ MIP: 本数・ノード・秒の和・最長 (秒)・解あり | SUBCAP 止め (葉・重さか T・待ち・B で後回し) |")
    print("|---|---|---|---:|---:|---:|---:|---:|---|---|")
    for r in sorted(rs, key=lambda r: (r["tag"], r["seed"])):
        res = r["result"]
        log = readlog(r["tag"], p, r["seed"])
        # 根のヒューリスティクスの終わり (最初のもの)
        m = re.search(r"after root heuristics: incumbent (\S+?)\s*\(([\d.]+)s\)", log)
        rt, rinc = (float(m.group(2)), m.group(1)) if m else (None, "-")
        # 根の終わりは外側の時計 (前処理を含む) の木の始まり
        m2 = re.search(r"TRSTAT: tree start outer ([\d.]+)", log)
        if m2:
            rt = float(m2.group(1))
        rinc = rinc.replace("Some(", "").rstrip(")")
        # 根のサブ MIP: 木に入る前 (最初の `after root heuristics` まで、と RINS などその後の根のもの) の `sub-MIP:` 行
        head = log
        tm = re.search(r"TRSTAT: tree start|MIP: node \d+", log)
        if tm:
            head = log[:tm.start()]
        subs = re.findall(r"(?m)^MIP:\s+(?:sub-MIP|parallel sub-MIP \(([^)]*)\)): (?:\d+ free columns of \d+, )?status (\w+), nodes (\d+), objective (\S+?), ([\d.]+)s", head)
        n = len(subs)
        nodes = sum(int(s[2]) for s in subs)
        secs = [float(s[4]) for s in subs]
        sol = sum(1 for s in subs if s[3] != "None")
        jn = sum(int(x) for x in re.findall(r"TREE3_JOINCAP: stopping (\d+) of", log))
        scs = f"{log.count('SUBCAP n: stopping')}・{log.count('SUBCAP w: stopping') + log.count('SUBCAP T: stopping')}・{jn + log.count('SUBCAP J: stopping')}・{log.count('SUBCAP B: deferring')}"
        sub = f"{n}・{nodes}・{sum(secs):.1f}・{max(secs, default=0):.1f}・{sol}" if n else "-"
        print(f"| {r['tag']} | {r['seed']} | {res.get('status')} | {g(res.get('time'))} | {g(res.get('objective'))} | {g(res.get('bound'))} | {g(rt)} | {rinc} | {sub} | {scs} |")
    print()
