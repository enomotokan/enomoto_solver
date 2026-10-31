"""neos-2978193-inde のログから時間の配分を出す: 再スタートの時刻 (最初の根の長さ)、再スタート後の根のヒューリスティクス・木の始め・木の終わり
(再スタート後の時計)、木のノード (主の数え方と全スレッド)、ノードの LP 秒、強分枝の秒、最適値 (-2.38806) が出た時刻と出どころ、全体の秒。
使い方: python stat.py <ログ>..."""
import re, sys
for f in sys.argv[1:]:
    s = open(f).read()
    rst = re.findall(r"^MIP: restart \d+ \(.*\) \(([\d.]+)s\)", s, re.M)
    rh = re.findall(r"^MIP: after root heuristics: .*\(([\d.]+)s\)", s, re.M)
    ts = re.findall(r"^MIP: tree search with \d+ threads \(([\d.]+)s\)", s, re.M)
    fe = re.findall(r"^MIP: finish start \w+ \(([\d.]+)s\)", s, re.M)
    wn = re.findall(r"worker nodes (\d+)", s)
    nd = re.findall(r"^MIP: node diag: nodes (\d+) node LPs \d+ mean rows ([\d.]+) iters \d+ \([\d.]+/LP\) LP secs ([\d.]+)", s, re.M)
    sb = re.findall(r"sb_secs ([\d.]+)", s)
    opt = re.search(r"^MIP: new incumbent -2\.38806\d*e0 at node \d+ \(([\d.]+)s\) \[([^\]]*)\]", s, re.M)
    first = s.split("MIP: restart 1 (")[0]
    er = re.search(r"^MIP: early restart: .* after (.+?) \(\d", first, re.M)
    subs = re.findall(r"^MIP:   (?:parallel )?sub-MIP(?: \((\w+)\))?: \d+ free columns of \d+, status \w+, nodes (\d+), objective [^,]*(?:\([^)]*\))?, ([\d.]+)s|^MIP:   parallel sub-MIP \((\w+)\): status \w+, nodes (\d+), objective .*, ([\d.]+)s$", first, re.M)
    subs = [(a or d or "-", b or e, c or g) for a, b, c, d, e, g in subs]
    sub_txt = " ".join(f"{n[:4]}:{float(t):.1f}" for n, _, t in subs if float(t) >= 0.2)
    tot = re.findall(r'"time": ([\d.]+)', s)
    st = re.findall(r'"status": "(\w+)"', s)
    print(f"{f.split('/')[-1]:44s} {st[-1] if st else '?':8s} time {float(tot[-1]) if tot else -1:5.1f} restart@{'+'.join(rst) or '-':10s} "
          f"rootheur {'/'.join(rh):12s} tree {ts[-1] if ts else '-'}..{fe[-1] if fe else '-'} nodes(main {nd[-1][0] if nd else '-'}, workers {wn[-1] if wn else '-'}) "
          f"nodeLP {nd[-1][2] if nd else '-'}s rows {nd[-1][1] if nd else '-'} sb {sb[-1] if sb else '-'}s opt@{opt.group(1)+'s '+opt.group(2) if opt else '-'} early:{er.group(1) if er else '-'} root1sub[{sub_txt}]")
