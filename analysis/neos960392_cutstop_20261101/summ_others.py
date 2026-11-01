"""ほかの問題の表: 設定ごとに各回の「時間 (T は時間切れ、括弧は暫定解) / 最初の根のカットの後の下界 / ラウンドの数・止めた理由 / ループの終わりの時刻」。
使い方: python summ_others.py [設定,...]"""
import json, re, sys
from pathlib import Path
D = Path(__file__).resolve().parent
cfgs = sys.argv[1].split(",") if len(sys.argv) > 1 else ["def", "fl3", "lcsf", "wkd"]
res = {}
for line in (D / "results.jsonl").read_text().splitlines():
    r = json.loads(line)
    res.setdefault((r["prob"], r["cfg"]), []).append(r)
probs = sorted({p for p, _ in res if p != "neos-960392"}, key=lambda p: min(r["wall"] for c in cfgs for r in res.get((p, c), [{"wall": 1e9}])))
def cell(r):
    t = (D / "logs" / f"{r['prob']}_{r['cfg']}_{r['rep']}.log").read_text(errors="replace")
    st = f"{r['time']:.1f}" if r.get("status") == "optimal" else f"T ({r.get('objective')})"
    m = re.search(r"cut loop done: (\d+) cuts added, (\d+) in LP.*obj (\S+) -> (\S+)", t)
    lb = f"{float(m.group(4)):.6g}" if m else "-"
    # 最初のループのラウンド (再スタートの前): 最初の「cut loop done」までの行
    head = t.split("cut loop done")[0]
    nr = len(re.findall(r"MIP: cut round \d+: \d+ cuts", head))
    nf = len(re.findall(r"LP status \w+, removed", head))
    stop = ("flat" if "ENOMOTO_T_MIP_CUT_FLAT_STOP" in head else "lpcap" if "ENOMOTO_MIP_CUT_LP_CAP_STOP" in head else
            "work" if re.search(r"work cap check: .* -> stop", head) else "time" if re.search(r"time cap check: .* -> stop", head) else
            "intobj" if "integral objective, LP bound" in head else "")
    mr = re.search(r"cut round \d+: \d+ cuts .*\(([\d.]+)s\)\n(?!.*cut round)", head)
    last = re.findall(r"cut round \d+: \d+ cuts .*\(([\d.]+)s\)", head)
    return f"{st} / {lb} / {nr}r{'+' + str(nf) + 'f' if nf else ''}{' ' + stop if stop else ''} / {float(last[-1]):.1f}s" if last else f"{st} / {lb} / 0r"
print("| 問題 | " + " | ".join(cfgs) + " |")
print("|---|" + "---|" * len(cfgs))
for p in probs:
    print(f"| {p} | " + " | ".join("<br>".join(cell(r) for r in sorted(res.get((p, c), []), key=lambda r: r["rep"])) for c in cfgs) + " |")
