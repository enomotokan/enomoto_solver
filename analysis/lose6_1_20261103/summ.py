"""lose6_1 (2026-11-03): ENOMOTO のログ (logs/<問題>_<設定>_<回>.log) から時間の内訳を取り出して表にする。
使い方: python summ.py [問題,...] [設定,...]   (省略で全部)。再スタートの後のログの時刻は再スタートからの秒なので、再スタートの時刻を足して通しの秒にする。
列: 根の LP の同時実行の勝ち (主の根)、主の根のカットのループの終わり (秒・値)、根のヒューリの終わり、裏のカット (ラウンド・待ち・終わりの値)、
再スタート (時刻と固定)、木の始め、最適値を見つけた時刻と見つけたもの、ノード、結果。"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
OPT = {"h80x6320d": 6382.09905, "neos-2978193-inde": -2.388061686}
probs = sys.argv[1].split(",") if len(sys.argv) > 1 and sys.argv[1] else None
cfgs = sys.argv[2].split(",") if len(sys.argv) > 2 else None
res = {}
for l in (D / "res.jsonl").read_text().splitlines():
    r = json.loads(l); res[(r["problem"], r["cfg"], r["run"])] = r
T = lambda s: float(re.findall(r"([0-9.]+)s\)", s)[-1])


def one(P, c, i):
    log = (D / "logs" / f"{P}_{c}_{i}.log").read_text().splitlines()
    off = 0.0; seg = 0  # 再スタートの回数 (区間)
    row = {"race": [], "cutend": [], "heur": [], "bg": [], "restart": [], "tree": None, "opt": None, "stop": []}
    for k, s in enumerate(log):
        if s.startswith("MIP root race:"):
            # 前処理の LP の探りの後の根 (「solving the root LP」の後) の勝ちだけを取る
            if any("solving the root LP" in x for x in log[max(0, k - 2):k]):
                row["race"].append(s.split(":", 1)[1].strip().split(" at")[0].replace(" won", ""))
        elif s.startswith("MIP: cut loop done:"):
            # 主のループの終わりだけを取る (すぐ後に「early root reduced costs stored (…, 秒)」が出る。裏のカットのループの終わりは捨てる)
            m = re.search(r"obj (\S+) -> (\S+)", s)
            for x in log[k + 1:k + 10]:
                if "early root reduced costs stored" in x:
                    row["cutend"].append((T(x) + off, float(m.group(2)))); break
        elif s.startswith("MIP: after root heuristics:"):
            row["heur"].append(T(s) + off)
        elif "background cuts: determ: waited" in s:
            m = re.search(r"waited ([0-9.]+)s .*?, (\w+), background end ([^)]*)\), (\d+) rounds.*obj Some\(([-0-9.e]+)\)", s)
            row["bg"].append((int(m.group(4)), float(m.group(1)), m.group(2), float(m.group(5)), T(s) + off))
        elif re.match(r"MIP: restart \d+ \(", s):
            t = T(s) + off
            m = re.search(r"\((\d+) of (\d+) integer columns newly fixed", s)
            row["restart"].append((t, int(m.group(1)) if m else None))
            off = t
        elif s.startswith("MIP: tree search with"):
            row["tree"] = T(s) + off
        elif "new incumbent" in s and row["opt"] is None:
            m = re.search(r"new incumbent (\S+) at node (\d+) \(([0-9.]+)s\) \[(.*)\]", s)
            if m and abs(float(m.group(1)) - OPT[P]) <= 1e-6 * max(1, abs(OPT[P])):
                row["opt"] = (float(m.group(3)) + off, m.group(4), int(m.group(2)))
        elif "LOSE6_1" in s:
            row["stop"].append(s.split(":", 2)[2].strip()[:80])
    return row


rows = []
for (P, c, i), r in sorted(res.items(), key=lambda kv: (kv[0][0], kv[0][1], kv[0][2])):
    if c == "highs" or (probs and P not in probs) or (cfgs and c not in cfgs):
        continue
    if not (D / "logs" / f"{P}_{c}_{i}.log").exists():
        continue
    x = one(P, c, i)
    f = lambda v, d=1: "-" if v is None else f"{v:.{d}f}"
    st = "最適" if r["status"] == "optimal" else ("時間切れ" if r["status"] == "time_limit" else r["status"])
    rows.append("| " + " | ".join([
        P, c, str(i), st, f(r["time"]),
        "/".join(x["race"]),
        "; ".join(f"{f(t)}s {v:.6g}" for t, v in x["cutend"]),
        "; ".join(f(t) for t in x["heur"]),
        "; ".join(f"{n}r 待ち{w:.2f} {why} {v:.6g}" for n, w, why, v, _ in x["bg"]),
        "; ".join(f"{f(t)}s ({n})" for t, n in x["restart"]),
        f(x["tree"]),
        f"{x['opt'][0]:.1f} {x['opt'][1]}" if x["opt"] else "-",
        f"{r.get('root_bound') or float('nan'):.6g}", str(r.get("nodes")),
    ]) + " |")
print("| 問題 | 設定 | 回 | 結果 | 時間 | 根の LP の勝ち (区間ごと) | 主の根のカットの終わり (秒・値) | 根のヒューリの終わり | 裏のカット (ラウンド・待ち・理由・値) | 再スタート (秒・固定) | 木の始め | 最適解 (秒・出元) | 根の下界 | ノード |")
print("|" + "---|" * 14)
print("\n".join(rows))
