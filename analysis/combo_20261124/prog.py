"""分担ごとの進み: python analysis/combo_20261124/prog.py → 済んだ回数・残りの見積り (時間)・組み替えの目安 (README 7 節)。
- 残りの見積り = 残りの回の plan.py の見積り (final_20261118 の wall、なければ 630 秒) × その分担の実績の比 (済んだ回の wall の合計 / その回の見積りの合計)。
- 取り込んだ res_s*.jsonl (持ち主と手伝い) と takeover.json を見る。最新を見るには先に git で取り込む。"""
import glob, json
from pathlib import Path
D = Path(__file__).resolve().parent
TL = 600
W = {}
for f in glob.glob(str(D.parent / "final_20261118" / "res_*_s*.jsonl")):
    for l in open(f, encoding="utf-8"):
        if not l.strip():
            continue
        r = json.loads(l)
        th = {"def_t1": 1, "def": 4}.get(r["cfg"])
        st = str(r["result"].get("status") or "").lower()
        if th and (r.get("tl") == 600 or st in ("optimal", "infeasible")):
            if (r["problem"], th) not in W or r.get("tl") == 600:
                W[(r["problem"], th)] = r["wall"]
def est(p, cfg): return W.get((p, 1 if cfg.endswith("_t1") else 4), TL + 30.0)
SH = json.loads((D / "shards.json").read_text())["shards"]
TO = json.loads((D / "takeover.json").read_text()) if (D / "takeover.json").exists() else {}
print("分担  済み/回数  実績の比  残りの見積り(時間)  手伝いに渡した組")
for k, runs in enumerate(SH):
    rows = {}; oom = {}
    for f in (D / f"res_s{k}.jsonl", D / f"res_s{k}_tail.jsonl"):
        if f.exists():
            for l in f.read_text().splitlines():
                if l.strip():
                    r = json.loads(l)
                    key = (r["problem"], r["cfg"])
                    if r["result"].get("status") not in ("crash", "fetch_error"):
                        rows[key] = r
                    elif r["result"].get("status") == "crash" and r["result"].get("returncode") == -9:
                        # run.py と同じく、OOM が 2 回そろった回は済んだとみる (wall はその回の最後の行)
                        oom[key] = oom.get(key, 0) + 1
                        if oom[key] >= 2: rows[key] = r
    done = [(p, c) for p, c, _, _ in runs if (p, c) in rows]
    e_done = sum(est(p, c) for p, c in done)
    ratio = sum(rows[x]["wall"] for x in done) / e_done if e_done else 1.0
    left = sum(est(p, c) for p, c, _, _ in runs if (p, c) not in rows) * ratio / 3600
    print(f"{k:>4}  {len(done):>3}/{len(runs):<3}  {ratio:8.2f}  {left:18.1f}  {TO.get(str(k), 0)}")
