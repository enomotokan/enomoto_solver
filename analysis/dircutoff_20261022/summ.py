# ログの要点: 根のループごとの (ラウンド数, 最後の目的値, 時刻) と RESULT
import sys, re, glob
for fn in sys.argv[1:]:
    loops=[]; cur=[]
    for l in open(fn, errors='replace'):
        m=re.match(r"MIP: cut round (\d+): \d+ cuts .* obj (\S+) \((\S+)s\)", l)
        if m: cur.append((int(m.group(1)), float(m.group(2)), m.group(3)))
        if l.startswith("MIP: cut loop done"):
            if cur: loops.append(f"{len(cur)}r {cur[-1][1]:.6g} @{cur[-1][2]}")
            cur=[]
    out=fn.replace('.err','.out'); res=''
    try: res=[l for l in open(out) if 'RESULT' in l][0].split('RESULT ')[1].strip()
    except Exception: pass
    print(fn.split('/')[-1][:-4], '|', '; '.join(loops[:4]), '|', re.sub(r'(\.\d{3})\d+', r'\1', res))
