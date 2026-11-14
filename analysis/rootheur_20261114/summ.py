"""RHSTAT の行を集計する: python summ.py 名前 [名前...] (logs/名前/*.log.gz)。
問題ごとに: 根の始まり (最初の RHSTAT main)・終わり (最後の RHSTAT main/par)、ヒューリスティクスごとの秒の和・LP 反復・改善の数、
暫定解の出どころ。全体で: ヒューリスティクスごとの秒の和・回数・改善の数・「5 秒以上使って改善なし」の回数と秒。"""
import gzip, sys
from collections import defaultdict
from pathlib import Path
D = Path(__file__).resolve().parent
def parse(f):
    ev = []
    for l in gzip.open(f, "rt", errors="replace"):
        if l.startswith("RHSTAT "):
            a = l.rstrip("\n").split("\t"); kind = a[0].split()[1]
            if kind in ("main", "par"):
                name, secs, it, found, z0, z1, at, rs = a[1], float(a[2]), int(a[3]), a[4] == "1", a[5], a[6], float(a[7]), int(a[8])
                ev.append(dict(kind=kind, name=name, secs=secs, it=it, found=found, imp=found, at=at, rs=rs))
            elif kind == "inc":
                ev.append(dict(kind="inc", name=a[1], z=float(a[2]), at=float(a[3]), node=int(a[4]), rs=int(a[5])))
        elif l.startswith("RESULT"):
            ev.append(dict(kind="result", res=l.split()[1:]))
    return ev
def main(names, long=5.0):
    tot = defaultdict(lambda: [0, 0.0, 0, 0, 0.0, 0])  # 回数, 秒, 改善, 長くて無, その秒, LP 反復
    rows = []
    for nm in names:
        for f in sorted((D / "logs" / nm).glob("*.log.gz")):
            ev = parse(f); prob = f.name.split(".")[0]
            mains = [e for e in ev if e["kind"] in ("main", "par")]
            if not mains: rows.append((prob, nm, 0, 0, 0, "", "")); continue
            per = defaultdict(lambda: [0.0, 0, 0])
            for e in mains:
                k = ("par:" if e["kind"] == "par" else "") + e["name"]
                t = tot[k]; t[0] += 1; t[1] += e["secs"]; t[2] += e["imp"]; t[5] += e["it"]
                if e["secs"] >= long and not e["imp"]: t[3] += 1; t[4] += e["secs"]
                per[k][0] += e["secs"]; per[k][1] += e["imp"]; per[k][2] += e["it"]
            main_secs = sum(e["secs"] for e in mains if e["kind"] == "main")
            waste = sorted(((v[0], k) for k, v in per.items() if not k.startswith("par:") and v[1] == 0 and v[0] >= 1.0), reverse=True)
            src = [(e["name"], round(e["at"], 1)) for e in ev if e["kind"] == "inc" and e["node"] <= 1]
            res = [e["res"] for e in ev if e["kind"] == "result"]
            rows.append((prob, nm, round(mains[0]["at"] - mains[0]["secs"], 1), round(max(e["at"] for e in mains), 1), round(main_secs, 1),
                         " ".join(f"{k}={s:.1f}" for s, k in waste[:4]), " ".join(f"{n}@{a}" for n, a in src[-3:]), res[-1][:2] if res else "?"))
    print("| 問題 | 組 | 根のヒューリスティクスの始まり (秒) | 終わり | 主のスレッドの秒の和 | 1 秒以上使って改善なし (主) | 根の暫定解の出どころ (最後の 3 つ) | 結果 |")
    print("|---|---|---|---|---|---|---|---|")
    for r in sorted(rows, key=lambda r: -r[4] if len(r) > 4 else 0):
        print("| " + " | ".join(map(str, r)) + " |")
    print()
    print(f"| ヒューリスティクス | 回数 | 秒の和 | 改善した回 | {long:g} 秒以上で改善なしの回 | その秒 | LP 反復 |")
    print("|---|---|---|---|---|---|---|")
    for k, v in sorted(tot.items(), key=lambda kv: -kv[1][1]):
        print(f"| {k} | {v[0]} | {v[1]:.1f} | {v[2]} | {v[3]} | {v[4]:.1f} | {v[5]} |")
if __name__ == "__main__":
    main(sys.argv[1:])
