"""CUTRND: res*.jsonl の各回について、ログから根のカットのループ (最初の根) の止まり方・ラウンド数・経過・下界、裏のカットの後の下界を並べる。
使い方: python analysis/cutrnd_20261112/summ.py [res.jsonl ...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
files = [Path(a) for a in sys.argv[1:]] or [D / "res.jsonl"]


def rootinfo(log):
    if not log.exists():
        return {}
    t = log.read_text(errors="replace")
    r = {}
    rounds = re.findall(r"cut round (\d+): \d+ cuts .*?obj ([-+0-9.e]+) \(([0-9.]+)s\)", t)
    m = re.search(r"cut loop done: .*?obj ([-+0-9.e]+) -> ([-+0-9.e]+)", t)
    if m:
        # 最初のループの終わりまでのラウンド
        end = t.find("cut loop done")
        rr = re.findall(r"cut round (\d+): \d+ cuts .*?obj ([-+0-9.e]+) \(([0-9.]+)s\)", t[:end])
        r["rounds"] = len(rr)
        r["loop_end_s"] = float(rr[-1][2]) if rr else None
        r["root_cut"] = float(rr[-1][1]) if rr else None
    b = re.search(r"background cuts: determ: waited .*?obj Some\(([-+0-9.e]+)\)", t)
    if b:
        r["bg"] = float(b.group(1))
    r["ext"] = t.count("CUTRND e:") and t.count("-> extend")
    r["s_stop"] = t.count("CUTRND s:")
    tr = re.search(r"MIP: ([0-9.]+)s nodes", t)
    if tr:
        r["tree_s"] = float(tr.group(1))
    return r


for f in files:
    for l in f.read_text().splitlines():
        x = json.loads(l)
        res = x["result"]
        log = D / "logs" / f"{x['problem']}_{x['build']}_{x['cfg']}_s{x['seed']}_{x['tl']}.log"
        ri = rootinfo(log)
        print(f"{x['problem']:18s} {x['cfg']:5s} s{x['seed']} {x['tl']:4d} {res.get('status','?'):10s} t={res.get('time') or 0:7.1f} obj={res.get('objective')} bnd={res.get('bound')} "
              f"rounds={ri.get('rounds')} loop_end={ri.get('loop_end_s')} root={ri.get('root_cut')} bg={ri.get('bg')} ext={ri.get('ext')} s={ri.get('s_stop')} rss={x.get('max_rss_mb')}")
