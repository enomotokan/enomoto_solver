"""60 秒の診断のログ (*_def_60s_t4.err.gz) の最後の POOL3_DIAG の行から型を分ける。
- 「各スレッドの最小と全体の最小の差」= (各スレッドの下界 (手元 + 処理中) の最小 − 全体の最小) / (打ち切り値 − 全体の最小)。
  大きければ全体の最小のノードは共有の待ち行列にだけあり、どのスレッドも手を付けていない (置き去り型)。
- 「最小ちょうど」= 共有の待ち行列のうち下界が全体の最小に等しいノードの数 (多ければ LP の値が分枝で上がらない平ら型)。"""
import glob, gzip, re
print("| 問題 | 処理ノード | 共有の待ち行列 | うち下界が最小ちょうど | 各スレッドの最小と全体の最小の差 (ギャップ比) | 処理したノードのうち下界が全体の最小 (最後の 10 秒) |")
print("|---|---:|---:|---:|---:|---:|")
for f in sorted(glob.glob("*_def_60s_t4.err.gz")):
    p = f[: -len("_def_60s_t4.err.gz")]
    last = None
    for l in gzip.open(f, "rt", errors="replace"):
        if l.startswith("POOL3_DIAG"):
            last = l
    if not last:
        continue
    rep = float(re.search(r"reported (\S+)", last).group(1)); inc = float(re.search(r" inc (\S+)", last).group(1))
    nn = re.search(r"nodes since last (\d+) at the global min (\d+)", last)
    m = re.search(r"len (\d+) hist.*?\[([^\]]*)\]", last)
    sl = [float(x.split("/")[0]) for x in re.findall(r" s\d+ (\S+)", last)]
    sl = [v for v in sl if v != float("inf")]
    tot = 0
    for l in open(p + "_def_60s_t4.out"):
        if l.startswith("{"):
            import json; tot = json.loads(l)["nodes"]
    gap = inc - rep if inc - rep > 0 else float("nan")
    d = (min(sl) - rep) / gap if sl and gap == gap else float("nan")
    ln, h0 = (m.group(1), m.group(2).split(",")[0].strip()) if m else ("-", "-")
    print(f"| {p} | {tot} | {ln} | {h0} | {d:.3f} | {nn.group(2)} / {nn.group(1)} |")
