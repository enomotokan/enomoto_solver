"""combo3 s2 (240 問・600 秒、cd10d12f7) で上限を超えた回の一覧: python list.py → 標準出力 (list.txt)。
超えた分を、求解の中 (result.time − 600) と外 (wall − result.time: 読み込み・後始末・プロセスの終わり) に分ける。
900 秒で止められた回 (status killed) は別に出す。超えた回 = wall > 605 か result.time > 600.5。"""
import json, glob, collections
from pathlib import Path
D = Path(__file__).resolve().parents[1] / "combo3_20261126" / "s2"
rows, killed = [], []
for f in sorted(glob.glob(str(D / "res_s*.jsonl"))):
    for l in open(f):
        r = json.loads(l)
        res = r["result"] if isinstance(r["result"], dict) else {}
        t, w = res.get("time") or 0.0, r.get("wall") or 0.0
        if str(res.get("status")) == "killed" or (w > 890 and t == 0):
            killed.append((r["problem"], r["cfg"], w)); continue
        if w > 605 or t > 600.5:
            rows.append(dict(p=r["problem"], c=r["cfg"], w=w, t=t, inn=max(0.0, t - 600), out=max(0.0, w - t)))
print(f"超えた回 {len(rows)} (wall > 605 か time > 600.5)、900 秒で止められた回 {len(killed)}: {killed}\n")
by = collections.defaultdict(list)
for x in rows: by[x["p"]].append(x)
print("| 問題 | 回 | 中 (time − 600) の最大 | 外 (wall − time) の最大 | 設定 |")
print("|---|---:|---:|---:|---|")
for p, xs in sorted(by.items(), key=lambda kv: -max(x["inn"] + x["out"] for x in kv[1])):
    print(f"| {p} | {len(xs)} | {max(x['inn'] for x in xs):.1f} | {max(x['out'] for x in xs):.1f} | {' '.join(sorted(x['c'] for x in xs))} |")
ni = sum(1 for x in rows if x["inn"] > 1.0); no = sum(1 for x in rows if x["out"] > 5.0)
print(f"\n中で 1 秒超: {ni} 回、外で 5 秒超: {no} 回")
