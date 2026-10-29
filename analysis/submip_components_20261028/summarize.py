"""`ENOMOTO_MIP_DEBUG_SUBMIP_COMPONENTS` のログ (SUBMIP_COMP の行) から、サブ MIP ごとの連結成分と結果を表にする。

使い方: python summarize.py <ログ ...> [--all]
サブ MIP 1 本 = 前処理の前の行 1 つ + 前処理の後の行 1 つ + 結果の行 1 つ (同じヒューリスティクスの名前の中では出した順に対応させる)。
成分は 2 列以上のものだけ数える (1 列だけの成分は、ほかの列と行を共有しないので分枝を掛け合わせない)。"""
import re, sys
from collections import defaultdict, deque
from pathlib import Path

PAT = re.compile(r"SUBMIP_COMP: (.*?) (before presolve|after presolve(?: \(no reduction\))?|result): (.*)")
COMP = re.compile(r"(as is|top 1% \(\d+ int cols\) dropped|top 5% \(\d+ int cols\) dropped) (\d+) components \(largest (\d+), singletons (\d+), top \[([\d,]*)\]\)")


def parse_comp(s):
    head = re.match(r"rows (\d+), free cols (\d+) \(int (\d+)\)", s)
    out = {"rows": int(head.group(1)), "free": int(head.group(2)), "int": int(head.group(3))}
    for m in COMP.finditer(s):
        key = {"as is": "asis"}.get(m.group(1), "top1" if "1%" in m.group(1) else "top5")
        n, largest, single = int(m.group(2)), int(m.group(3)), int(m.group(4))
        out[key] = (n - single, largest)  # 2 列以上の成分の数、最大の成分の列の数
    return out


def records(path):
    q = defaultdict(deque)  # 名前 -> まだ結果のないサブ MIP
    recs = []
    for line in open(path, encoding="utf-8", errors="replace"):
        m = PAT.match(line)
        if not m:
            continue
        label, stage, rest = m.groups()
        if stage == "before presolve":
            r = {"label": label or "(tree)", "before": parse_comp(rest)}
            q[label].append(r)
            recs.append(r)
        elif stage.startswith("after presolve"):
            for r in q[label]:
                if "after" not in r:
                    r["after"] = parse_comp(rest)
                    break
        else:
            if q[label]:
                r = q[label].popleft()
                st = re.match(r"status (\w+), nodes (\d+), ([\d.]+)s", rest)
                r["status"], r["nodes"], r["time"] = st.group(1), int(st.group(2)), float(st.group(3))
    return recs


def fmt_c(c, key):
    if not c or key not in c:
        return "-"
    k, largest = c[key]
    return f"{k} ({100 * largest / max(1, c['free']):.0f}%)"


show_all = "--all" in sys.argv
print("| 問題 | サブ MIP | 前処理後 行・自由な列 | 成分 (最大の割合) | 上位 1% を外す | 上位 5% を外す | 前処理前の成分 | 結果 | ノード | 秒 |")
print("|---|---|---|---|---|---|---|---|---:|---:|")
tot = defaultdict(int)
for f in sys.argv[1:]:
    if f.startswith("--"):
        continue
    name = Path(f).stem.rsplit("_", 2)[0]
    for r in records(f):
        a = r.get("after") or r["before"]
        split = a.get("asis", (0, 0))[0] >= 2
        split5 = a.get("top5", (0, 0))[0] >= 2
        tot["n"] += 1
        tot["split"] += split
        tot["split5"] += split5
        tot["limit"] += r.get("status") in ("NodeLimit", "TimeLimit")
        tot["limit_split"] += split and r.get("status") in ("NodeLimit", "TimeLimit")
        tot["limit_split5"] += split5 and r.get("status") in ("NodeLimit", "TimeLimit")
        if show_all or split or split5:
            print(f"| {name} | {r['label']} | {a['rows']}・{a['free']} | {fmt_c(a, 'asis')} | {fmt_c(a, 'top1')} | {fmt_c(a, 'top5')} | "
                  f"{fmt_c(r['before'], 'asis')} | {r.get('status', '?')} | {r.get('nodes', '')} | {r.get('time', '')} |")
print()
print(f"サブ MIP {tot['n']} 本: 前処理後に 2 列以上の成分が 2 つ以上 {tot['split']} 本、上位 5% を外すと {tot['split5']} 本。"
      f"ノード・時間の上限で止まった {tot['limit']} 本 (うち分かれる {tot['limit_split']} 本、上位 5% を外すと分かれる {tot['limit_split5']} 本)")
