# gdb のスタックを、スタックの回ごと・スレッドごとに enomoto のフレーム (最大 8 個) の並びにまとめる
import re, sys
skip = re.compile(r"futex|syscall|park|wait|sleep|condvar|rayon_core::registry|crossbeam|std::sys|epoll|pthread_cond|__GI|start_thread|clone3")
for fn in sys.argv[1:]:
    print("##", fn)
    cur = None; th = None; frames = []
    def flush():
        if th and frames:
            fr = [f for f in frames if "mip::" in f or "simplex::" in f or "presolve" in f or "model::" in f or "interior" in f]
            if fr: print("  ", th, " <- ".join(fr[:8]))
    for l in open(fn):
        if l.startswith("=== stack"):
            flush(); th=None; frames=[]; print(l.strip()); continue
        m = re.match(r"Thread \d+ .*\((LWP \d+)\)", l)
        if m:
            flush(); th = m.group(1); frames=[]; continue
        m = re.match(r"#\d+\s+(?:0x[0-9a-f]+ in )?(.+?) \(", l)
        if m:
            name = m.group(1)
            name = re.sub(r"::<[^>]*>", "", name).replace("<","").replace(">",""); name = re.sub(r"::h[0-9a-f]{16}$", "", name)
            name = name.replace("_core::", "").replace("enomoto_core::","")
            frames.append(name)
    flush()
