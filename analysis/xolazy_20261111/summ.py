"""XOLAZY 段階 0 の集計 (analysis/xolazy_20261111.md)。使い方: python summ.py A.jsonl [B.jsonl ...]
根の LP ごとに勝った側・内点法・押し出し・仕上げの時間と、その根の LP の後に再スタートしたかを数え、省ける上限を出す。
- 内点法が勝った根の LP: クロスオーバー (押し出し + 仕上げ + 採る手間) = t_end - t_ipm を省ける。
- 二段解法が勝った根の LP: 内点法が二段解法より先に終わっていれば t_win - t_ipm 早く値が出せた (クロスオーバーなしなら内点法が勝つ)。
「後に再スタート」= 同じ回でその根の LP の次の出来事 (rootlp・restart・tree の並び) に restart がある。"""
import json, sys
from collections import defaultdict

rows = []
for f in sys.argv[1:]:
    for l in open(f):
        if l.strip():
            rows.append(json.loads(l))
agg = defaultdict(float)
cnt = defaultdict(int)
per = []
tot_time = 0.0
for r in rows:
    ev = r.get("xolazy", [])
    t_total = r["result"].get("time") or 0.0
    tot_time += t_total
    t_tree = next((e["t"] for e in ev if e["ev"] == "tree"), None)
    for i, e in enumerate(ev):
        if e["ev"] != "rootlp":
            continue
        nxt = next((x for x in ev[i + 1:] if x["ev"] in ("restart", "tree", "rootlp")), None)
        rs = nxt is not None and nxt["ev"] == "restart"
        t_to_restart = nxt["t"] - e["t"] if rs else None
        w = e["winner"]
        save = 0.0
        push = clean = 0.0
        if w in ("ipm", "ipm_resolve") and e["t_ipm"] is not None:
            save = e["t_end"] - e["t_ipm"]
            if e["t_push"] is not None:
                push = e["t_push"] - e["t_ipm"]
                clean = (e["t_xo"] or e["t_win"]) - e["t_push"]
        elif w == "simplex" and e["t_ipm"] is not None:
            save = max(0.0, e["t_win"] - e["t_ipm"])
        k = ("restart" if rs else "norestart", w)
        cnt[k] += 1
        agg[k + ("secs",)] += e["secs"]
        agg[k + ("save",)] += save
        agg[k + ("push",)] += push
        agg[k + ("clean",)] += clean
        per.append((save, r["problem"], r["cfg"], r["rep"], e["kind"], e["restarts"], w, e["secs"], e["t_ipm"], push, clean, rs, t_to_restart, t_total, t_tree))
print(f"回 {len(rows)}、時間の和 {tot_time:.1f} 秒、根の LP {len(per)}")
print("| 後に再スタート | 勝った側 | 根の LP 数 | 秒の和 | 省ける上限の和 | 押し出し | 仕上げ |")
print("|---|---|---|---|---|---|---|")
for k in sorted(cnt):
    print(f"| {k[0]} | {k[1]} | {cnt[k]} | {agg[k + ('secs',)]:.2f} | {agg[k + ('save',)]:.2f} | {agg[k + ('push',)]:.2f} | {agg[k + ('clean',)]:.2f} |")
s_all = sum(p[0] for p in per)
s_rs = sum(p[0] for p in per if p[11])
print(f"省ける上限: 全部の根の LP {s_all:.2f} 秒 ({100 * s_all / max(tot_time, 1e-9):.2f}% of 全体)、後に再スタートした根の LP {s_rs:.2f} 秒 ({100 * s_rs / max(tot_time, 1e-9):.2f}%)")
print("\n省ける上限の大きい根の LP (上位 25):")
print("| 問題 | 種 | 種類 | 再スタート回 | 勝った側 | 根の LP 秒 | 内点法の終わり | 押し出し | 仕上げ | 省ける上限 | 後に再スタート (までの秒) | 全体の秒 | 木に入った時刻 |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
f2 = lambda v: "-" if v is None else f"{v:.2f}"
for p in sorted(per, reverse=True)[:25]:
    save, prob, cfg, rep, kind, rsn, w, secs, tipm, push, clean, rs, ttr, tt, ttree = p
    print(f"| {prob} | {rep} | {kind} | {rsn} | {w} | {secs:.2f} | {f2(tipm)} | {push:.2f} | {clean:.2f} | {save:.2f} | {'yes (' + f2(ttr) + ')' if rs else 'no'} | {tt:.1f} | {f2(ttree)} |")
