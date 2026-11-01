"""run.py の結果の集計。python summ.py > summ.txt
- 問題 × 設定: 3 回の秒 (時間切れは T(目的値))・目的値
- 設定ごと・深さごと: サブ MIP の本数・秒、解なし (objective=None) の本数・秒、解なしの打ち切り (status=TimeLimit かつ解なしで
  ノード上限未満) の本数、acc (呼び出し側の暫定解を良くした本数)
- 問題 × 設定: 深さ 1 の解なしのサブ MIP の秒 (3 回の平均) と acc の本数"""
import gzip, json, re, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
sys.path.insert(0, str(D))
from run import PROBS, CFGS
KV = re.compile(r"(\w+)=(\[[^\]]*\]|\S+)")


def parse(path):
    subs = defaultdict(dict)
    with gzip.open(path, "rt", errors="replace") as f:
        for line in f:
            if not line.startswith("MIP: SMT "):
                continue
            ev = line.split()[2]
            kv = dict(KV.findall(line))
            i = kv.get("id")
            if ev == "begin":
                subs[i].update(depth=int(kv["depth"]), kind=kv["kind"].strip("[]"), node_limit=int(kv["node_limit"]))
            elif ev == "end":
                subs[i].update(status=kv["status"], nodes=int(kv["nodes"]), obj=kv["objective"], secs=float(kv["secs"]))
            elif ev == "acc":
                subs[i]["acc"] = kv["accepted"] == "1"
    return [s for s in subs.values() if "secs" in s and "depth" in s]


def main():
    res = defaultdict(dict)
    for line in open(D / "results.jsonl"):
        e = json.loads(line)
        res[(e["prob"], e["cfg"])][e["rep"]] = e["res"] or {}
    cfgs = [c for c in CFGS if any(k[1] == c for k in res)]
    print("## 問題ごとの秒 (T は時間切れ、括弧は目的値)\n")
    print("| 問題 | " + " | ".join(cfgs) + " |")
    print("|---|" + "---|" * len(cfgs))
    for p in PROBS:
        row = []
        for c in cfgs:
            rr = res.get((p, c), {})
            cell = []
            for k in sorted(rr):
                r = rr[k]
                if r.get("status") == "optimal":
                    cell.append(f"{r['time']:.1f}")
                else:
                    o = r.get("objective")
                    cell.append(f"T({o:.6g})" if o is not None else "T(-)")
            row.append(" / ".join(cell))
        print(f"| {p} | " + " | ".join(row) + " |")
    # 平均 (時間切れは 60 秒)、解けた数
    print("\n| | " + " | ".join(cfgs) + " |")
    print("|---|" + "---|" * len(cfgs))
    for name, f in [("秒の和 (時間切れ 60)", lambda r: min(r.get("time", 60), 60) if r.get("status") == "optimal" else 60), ("解けた回数", lambda r: r.get("status") == "optimal")]:
        print(f"| {name} | " + " | ".join(f"{sum(f(r) for (p, cc), rr in res.items() if cc == c for r in rr.values()):.1f}" for c in cfgs) + " |")
    agg = defaultdict(lambda: defaultdict(float))
    per = defaultdict(lambda: defaultdict(float))
    for (p, c), rr in res.items():
        for k in rr:
            f = D / "logs" / c / f"{p}_r{k}.log.gz"
            if not f.exists():
                continue
            for s in parse(f):
                d = min(s["depth"], 3)
                a = agg[(c, d)]
                nosol = s["obj"] == "None"
                a["n"] += 1
                a["secs"] += s["secs"]
                a["acc"] += s.get("acc", False)
                if nosol:
                    a["nosol_n"] += 1
                    a["nosol_secs"] += s["secs"]
                    if s["status"] == "TimeLimit" and s["nodes"] < s["node_limit"]:
                        a["stopped"] += 1
                if d == 1:
                    q = per[(p, c)]
                    q["acc"] += s.get("acc", False)
                    if nosol:
                        q["nosol_secs"] += s["secs"]
                        q["nosol_n"] += 1
    print("\n## 設定 × 深さ (3 回の和。11 問)\n")
    print("| 設定 | 深さ | 本数 | 秒 | acc | 解なしの本数 | 解なしの秒 | うち解なしの打ち切り (TimeLimit) |")
    print("|---|---|---|---|---|---|---|---|")
    for c in cfgs:
        for d in (1, 2, 3):
            a = agg.get((c, d))
            if a:
                print(f"| {c} | {d}{'+' if d == 3 else ''} | {a['n']:.0f} | {a['secs']:.1f} | {a['acc']:.0f} | {a['nosol_n']:.0f} | {a['nosol_secs']:.1f} | {a['stopped']:.0f} |")
    print("\n## 問題 × 設定: 深さ 1 の解なしのサブ MIP の本数・秒 (3 回の和) / acc の本数\n")
    print("| 問題 | " + " | ".join(cfgs) + " |")
    print("|---|" + "---|" * len(cfgs))
    for p in PROBS:
        print(f"| {p} | " + " | ".join(f"{per[(p, c)]['nosol_n']:.0f} 本 {per[(p, c)]['nosol_secs']:.1f}s / {per[(p, c)]['acc']:.0f}" for c in cfgs) + " |")


if __name__ == "__main__":
    main()
