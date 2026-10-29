"""prof_run.py のログから段階ごとの時間を表にする。

使い方: python parse.py ours  logs/base_<名前>_1.log ...   (ENOMOTO、時刻つき)
        python parse.py highs logs/highs_<名前>_1.log ...  (HiGHS)
        python parse.py table <タグ> [<タグ> ...]           (logs/<タグ>_*_1.log と logs/highs_*_1.log の Markdown の表)

ENOMOTO の段階は「その段階の終わりに出る行」で区切る (前の目印からその行までをその段階の時間とする)。
再スタートごとに周 (round) を分け、周 0 の前処理の内訳・各周の根の時間・木の時間を出す。
"""

from __future__ import annotations

import json
import re
import sys
from collections import OrderedDict
from pathlib import Path

TS = re.compile(r"^\[\s*([0-9.]+)\] (.*)$")

# (段階名, その段階の終わりの行の正規表現)。上から順に当てる
MARKERS = [
    ("lp_probe", re.compile(r"^MIP: LP probing: root LP")),
    ("presolve", re.compile(r"^MIP: presolve: rows")),
    ("probe", re.compile(r"^MIP: probing: \d+ of \d+ binaries probed")),
    ("equiv", re.compile(r"^MIP: probing: \d+ equivalent binary pairs")),
    ("clique_ext", re.compile(r"^MIP: clique extension:")),
    ("symmetry", re.compile(r"^MIP: symmetry: (\d+ generators|no |none|skipped|detection)")),
    ("pre_rootlp", re.compile(r"^MIP: solving the root LP")),
    ("root_lp", re.compile(r"^MIP: root LP -?[0-9]")),
    ("cuts", re.compile(r"^MIP: cut loop done")),
    ("root_heur", re.compile(r"^MIP: after root heuristics")),
    ("bg_cuts", re.compile(r"^MIP: background cuts: (\d+ rounds|.*not used|.*waited)")),
    ("pre_restart", re.compile(r"^MIP: restart \d+ \(")),
    ("pre_tree", re.compile(r"^MIP: tree search with")),
    ("finish", re.compile(r"^MIP: finish start")),
]
INC = re.compile(r"^MIP: new incumbent (\S+) at node (\d+) \(([0-9.]+)s\) \[([^\]]*)\]")


def parse_ours(path: Path) -> dict:
    lines = []
    result = None
    for l in path.read_text(encoding="utf-8", errors="replace").splitlines():
        if l.startswith("@@RESULT"):
            result = json.loads(l[len("@@RESULT "):])
            continue
        m = TS.match(l)
        if m:
            lines.append((float(m.group(1)), m.group(2)))
    rounds = [OrderedDict()]
    prev = 0.0
    in_tree = False
    incs = []
    restarts = []
    tree_threads = None
    for t, text in lines:
        if text.startswith("TREEPAR") or text.startswith("@@"):
            continue
        mi = INC.match(text)
        if mi:
            incs.append((t, float(mi.group(1)), mi.group(4)))
        for name, rx in MARKERS:
            if rx.match(text):
                if name == "finish":
                    name = "tree" if in_tree else "post_root"
                if name == "pre_tree":
                    in_tree = True
                    m = re.search(r"with (\d+) threads", text)
                    tree_threads = int(m.group(1)) if m else None
                d = rounds[-1]
                d[name] = d.get(name, 0.0) + (t - prev)
                prev = t
                if name == "pre_restart":
                    restarts.append(t)
                    rounds.append(OrderedDict())
                break
    total = result["time"] if result else (lines[-1][0] if lines else 0.0)
    rounds[-1]["post"] = rounds[-1].get("post", 0.0) + max(0.0, total - prev)
    obj = result.get("objective") if result else None
    t_opt = None
    if obj is not None:
        for t, z, lab in incs:
            if abs(z - obj) <= 1e-6 * max(1.0, abs(obj)):
                t_opt = (t, lab)
                break
    return {"rounds": rounds, "incs": incs, "result": result, "t_opt": t_opt, "restarts": restarts, "total": total, "tree_threads": tree_threads}


HROW = re.compile(r"^ ([A-Za-z ])\s+(\d+)\s+(\d+)\s+(\d+)\s+([0-9.]+)%\s+(\S+)\s+(\S+)\s+(\S+)\s+(\d+)\s+(\d+)\s+(\d+)\s+(\d+)\s+([0-9.]+)s")


def parse_highs(path: Path) -> dict:
    text = path.read_text(encoding="utf-8", errors="replace").splitlines()
    result = None
    presolve = None
    rows = []
    restarts = []
    for l in text:
        if l.startswith("@@RESULT"):
            result = json.loads(l[len("@@RESULT "):])
        m = re.match(r"^\s+([0-9.]+) \(Presolve\)", l)
        if m and presolve is None:
            presolve = float(m.group(1))
        m = HROW.match(l)
        if m:
            src = m.group(1).strip()
            nodes = int(m.group(2))
            rows.append({"src": src, "nodes": nodes, "lb": m.group(6), "ub": m.group(7), "cuts": int(m.group(9)), "iters": int(m.group(12)), "t": float(m.group(13))})
        if "restart" in l.lower() and "Model" not in l:
            restarts.append(l.strip())
    obj = result.get("objective") if result else None
    t_opt = None
    t_first = None
    root_lp = None
    t_root_end = None
    for r in rows:
        if r["ub"] not in ("inf", "-inf") and t_first is None:
            t_first = r["t"]
        try:
            if obj is not None and t_opt is None and abs(float(r["ub"]) - obj) <= 1e-6 * max(1.0, abs(obj)):
                t_opt = (r["t"], r["src"])
        except ValueError:
            pass
        if r["lb"] not in ("-inf", "inf") and root_lp is None:
            root_lp = r["t"]
        if r["nodes"] == 0:
            t_root_end = r["t"]
    return {"result": result, "presolve": presolve, "rows": rows, "t_opt": t_opt, "t_first": t_first, "t_rootlp": root_lp, "t_root_end": t_root_end, "restarts": restarts}


PRE = ["presolve", "lp_probe", "probe", "equiv", "clique_ext", "symmetry"]


def fmt(v):
    return "-" if v is None else f"{v:.2f}"


def summarize_ours(p: dict) -> dict:
    r0 = p["rounds"][0]
    out = {k: r0.get(k, 0.0) for k in PRE}
    out["pre_total"] = sum(out.values()) + r0.get("pre_rootlp", 0.0)
    out["root_lp"] = r0.get("root_lp", 0.0)
    out["cuts"] = r0.get("cuts", 0.0)
    out["root_heur"] = r0.get("root_heur", 0.0)
    out["bg_cuts"] = r0.get("bg_cuts", 0.0)
    out["restart_rounds"] = [sum(v for k, v in rd.items() if k not in ("tree", "pre_tree", "post")) for rd in p["rounds"][1:]]
    out["tree"] = sum(rd.get("tree", 0.0) for rd in p["rounds"])
    out["total"] = p["total"]
    return out


def table(tags: list[str]) -> str:
    logs = Path(__file__).parent / "logs"
    names = sorted({f.name[len(tags[0]) + 1:-6] for f in logs.glob(f"{tags[0]}_*_1.log")})
    out = []
    for tag in tags:
        out.append(f"\n### ENOMOTO ({tag})\n")
        out.append("| 問題 | 計 | 前処理計 | presolve | lp_probe | probe | equiv | clique_ext | symmetry | 根 LP | カット | 根ヒュー | 背景カット待ち | 再スタート (周の時間) | 木 | 最適解の時刻 (見つけた所) | 証明 |")
        out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|---:|---|---:|")
        for n in names:
            f = logs / f"{tag}_{n}_1.log"
            if not f.exists():
                continue
            p = parse_ours(f)
            s = summarize_ours(p)
            rr = ", ".join(f"{x:.2f}" for x in s["restart_rounds"]) or "-"
            to = f"{p['t_opt'][0]:.2f} ({p['t_opt'][1]})" if p["t_opt"] else "-"
            st = p["result"]["status"] if p["result"] else "?"
            proof = f"{s['total']:.2f}" if st == "optimal" else st
            out.append(f"| {n} | {s['total']:.2f} | {s['pre_total']:.2f} | " + " | ".join(fmt(s[k]) for k in PRE)
                       + f" | {fmt(s['root_lp'])} | {fmt(s['cuts'])} | {fmt(s['root_heur'])} | {fmt(s['bg_cuts'])} | {len(s['restart_rounds'])} ({rr}) | {fmt(s['tree'])} | {to} | {proof} |")
    out.append("\n### HiGHS\n")
    out.append("| 問題 | 計 | presolve | 根 LP の終わり | 根の終わり (ノード 0 の最後の行) | 最初の解 | 最適解の時刻 (Src) | 再スタート | ノード |")
    out.append("|---|---:|---:|---:|---:|---:|---|---:|---:|")
    for n in names:
        f = logs / f"highs_{n}_1.log"
        if not f.exists():
            continue
        h = parse_highs(f)
        to = f"{h['t_opt'][0]:.2f} ({h['t_opt'][1]})" if h["t_opt"] else "-"
        out.append(f"| {n} | {h['result']['time']:.2f} | {fmt(h['presolve'])} | {fmt(h['t_rootlp'])} | {fmt(h['t_root_end'])} | {fmt(h['t_first'])} | {to} | {len(h['restarts'])} | {h['result']['nodes']} |")
    return "\n".join(out)


def main() -> None:
    kind = sys.argv[1]
    if kind == "table":
        print(table(sys.argv[2:]))
        return
    for f in sys.argv[2:]:
        p = parse_ours(Path(f)) if kind == "ours" else parse_highs(Path(f))
        if kind == "ours":
            print(f, json.dumps(summarize_ours(p)), p["t_opt"], [(round(t, 2), z, l) for t, z, l in p["incs"]])
        else:
            print(f, {k: v for k, v in p.items() if k != "rows"})


if __name__ == "__main__":
    main()
