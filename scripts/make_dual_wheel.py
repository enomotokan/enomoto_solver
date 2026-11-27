"""基本の命令で作った wheel に、x86-64-v3 向けに作った同じ版の wheel の `_core` を `_core_v3` として足した wheel を作る。

    python scripts/make_dual_wheel.py BASE_WHEEL V3_WHEEL OUT_DIR

- 2 つの wheel はソース・版・タグが同じで、違うのは `-C target-cpu=x86-64-v3` だけのもの。
- 読み分けは `enomoto_solver/__init__.py` の `_enomoto_load_core` が、基本の `_core` の `cpu_x86_64_v3()` を見て行う。
- `_core` の名前は OS で違う (`_core.abi3.so`・`_core.cpython-313-x86_64-linux-gnu.so`・`_core.pyd`)。
  `_core.` を `_core_v3.` に替えた名前で入れる。
- `*.dist-info/RECORD` (sha256・大きさ) は作り直す。wheel の名前 (タグ) は BASE_WHEEL のまま。

配る wheel の作り方の全体は scripts/build_dist_wheel.sh、測った結果は analysis/buildopt_20261122/README.md。
"""
import base64
import hashlib
import os
import sys
import zipfile


def digest(b: bytes) -> str:
    return "sha256=" + base64.urlsafe_b64encode(hashlib.sha256(b).digest()).rstrip(b"=").decode()


def core_member(names):
    c = [n for n in names if n.startswith("enomoto_solver/_core.") and n.endswith((".so", ".pyd"))]
    if len(c) != 1:
        raise SystemExit(f"enomoto_solver/_core.* がちょうど 1 つではない: {c}")
    return c[0]


def main():
    if len(sys.argv) != 4:
        raise SystemExit(__doc__)
    bw, vw, out = sys.argv[1:4]
    if os.path.basename(bw) != os.path.basename(vw):
        raise SystemExit(f"wheel の名前 (版・タグ) が違う: {os.path.basename(bw)} と {os.path.basename(vw)}")
    with zipfile.ZipFile(vw) as z:
        so = core_member(z.namelist())
        v3 = z.read(so)
    v3name = so.replace("/_core.", "/_core_v3.", 1)
    os.makedirs(out, exist_ok=True)
    dst = os.path.join(out, os.path.basename(bw))
    if os.path.abspath(dst) == os.path.abspath(bw):
        raise SystemExit("OUT_DIR は BASE_WHEEL と別の所にする")
    files = []
    with zipfile.ZipFile(bw) as z:
        names = z.namelist()
        core_member(names)
        if any(n.startswith("enomoto_solver/_core_v3.") for n in names):
            raise SystemExit("BASE_WHEEL にもう _core_v3 がある")
        rec = next(n for n in names if n.endswith(".dist-info/RECORD"))
        for info in z.infolist():
            if info.filename != rec:
                files.append((info, z.read(info.filename)))
    v3info = zipfile.ZipInfo(v3name, date_time=files[0][0].date_time)
    v3info.external_attr = next(i.external_attr for i, _ in files if i.filename == so)
    files.append((v3info, v3))
    with zipfile.ZipFile(dst, "w", zipfile.ZIP_DEFLATED) as z:
        for info, b in files:
            info.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(info, b)
        z.writestr(rec, "".join(f"{i.filename},{digest(b)},{len(b)}\n" for i, b in files) + f"{rec},,\n")
    print(f"{dst} ({os.path.getsize(dst)} バイト、{so} と {v3name})")


if __name__ == "__main__":
    main()
