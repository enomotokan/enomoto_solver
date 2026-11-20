"""rhscan.jsonl と logs_rh/ (RHSTAT の行) の集計: python rhsumm.py > rhsumm.txt
- 根の LP の重さ (初めの根の LP の秒) で問題を分け、根のヒューリスティクスの秒 (主) と改善の回数を数える。
- Pump・3 つのダイビング (共有予算 RHEUR の対象) を別に数える。"""
import gzip, json, collections
from pathlib import Path
D = Path(__file__).resolve().parent
R = [json.loads(l) for l in open(D / "rhscan.jsonl")]
POOLED = {"feasibility pump", "fractional diving", "vector length diving", "coefficient diving"}
rows = []
for r in R:
    f = D / "logs_rh" / f"{r['problem']}.log.gz"
    if not f.exists() or "root_lp" not in r:
        continue
    hs = []
    for l in gzip.decompress(f.read_bytes()).decode().splitlines():
        if not l.startswith("RHSTAT"):
            continue
        t = l.split("\t")
        if t[0] != "RHSTAT main" and t[0] != "RHSTAT par":
            continue
        kind = t[0].split()[1]
        name, secs, iters, found = t[1], float(t[2]), int(t[3]), t[4]
        hs.append((kind, name, secs, iters, found not in ("0", "false", "False")))
    rows.append((r, hs))
print(f"問題 {len(rows)} (根の LP まで届いた問題)")
def bucket(s):
    return "a ≤1s" if s <= 1 else "b 1-10s" if s <= 10 else "c 10-60s" if s <= 60 else "d >60s"
B = collections.defaultdict(lambda: [0, 0.0, 0, 0.0, 0, 0, 0.0, 0, []])
for r, hs in rows:
    b = bucket(r["root_lp"][2])
    x = B[b]
    x[0] += 1
    main = [h for h in hs if h[0] == "main"]
    x[1] += sum(h[2] for h in main)
    x[2] += sum(h[4] for h in main) + sum(h[4] for h in hs if h[0] == "par")
    pooled = [h for h in main if h[1] in POOLED]
    x[3] += sum(h[2] for h in pooled)
    x[4] += sum(h[4] for h in pooled)
    if pooled:
        x[5] += 1
        x[8].append(f"{r['problem']}({sum(h[2] for h in pooled):.0f}s,{sum(h[4] for h in pooled)}改善)")
print("\n| 初めの根の LP の秒 | 問題 | 根のヒューリスティクスの主の秒の和 | 改善の回数 (主+並列) | Pump・ダイビングの秒 | その改善 | Pump・ダイビングが回った問題 |")
print("|---|---|---|---|---|---|---|")
for b in sorted(B):
    x = B[b]
    print(f"| {b} | {x[0]} | {x[1]:.0f} | {x[2]} | {x[3]:.0f} | {x[4]} | {x[5]} |")
print("\nPump・ダイビングが回った問題 (秒, 改善):")
for b in sorted(B):
    print(" ", b, ", ".join(B[b][8]))
# ヒューリスティクスごと
H = collections.defaultdict(lambda: [0, 0.0, 0, 0.0])
for r, hs in rows:
    for kind, name, secs, iters, imp in hs:
        if kind != "main":
            continue
        h = H[name]
        h[0] += 1; h[1] += secs; h[2] += imp
        if not imp: h[3] += secs
print("\n| ヒューリスティクス (主) | 回数 | 秒 | 改善 | 改善なしの秒 |")
print("|---|---|---|---|---|")
for name, h in sorted(H.items(), key=lambda kv: -kv[1][1]):
    if h[1] >= 1:
        print(f"| {name} | {h[0]} | {h[1]:.0f} | {h[2]} | {h[3]:.0f} |")
