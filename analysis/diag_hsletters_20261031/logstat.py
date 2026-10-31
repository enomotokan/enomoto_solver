"""ログから外側の求解の様子を集計する: 再スタートの回数、根のヒューリスティクスの時間 (再スタートごと)、
根の (並列でない・並列の) サブ MIP の個数・時間の和・ノードの和、最後の暫定解の時刻 (再スタートごとに時計が戻るので区間の番号つき)。
使い方: python logstat.py <ログ>..."""
import re, sys
for f in sys.argv[1:]:
    s = open(f).read()
    rs = len(re.findall(r"^MIP: restart \d+ \(", s, re.M))
    rh = [float(x) for x in re.findall(r"^MIP: after root heuristics: .*\(([\d.]+)s\)", s, re.M)]
    sub = [(int(n), float(t)) for n, t in re.findall(r"sub-MIP(?: \([^)]*\))?: (?:\d+ free columns of \d+, )?status \w+, nodes (\d+), objective [^,]*(?:\([^)]*\))?, ([\d.]+)s", s)]
    m = re.findall(r'"time": ([\d.]+)', s)
    print(f"{f.split('/')[-1]:34s} time {float(m[-1]) if m else -1:6.1f} restarts {rs} rootheur {'+'.join(f'{x:.1f}' for x in rh):24s} "
          f"subMIP n {len(sub):3d} secs {sum(t for _, t in sub):6.1f} nodes {sum(n for n, _ in sub)}")
