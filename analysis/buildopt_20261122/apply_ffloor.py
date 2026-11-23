"""(a) 移植できる速い floor・ceil・round (analysis/buildopt_20261122): python apply_ffloor.py ROOT
ROOT (作業木) の src の `.floor()`・`.ceil()`・`.round()` をすべて `.ffloor()`・`.fceil()`・`.fround()` に置き換え、src/ffloor.rs (trait FFloor) を足す。
SSE2 だけの命令 (cvttsd2si・cvtsi2sd) で、|x| < 2^52 なら x を 0 の向きに切り捨てた整数 t から作り、それ以外 (大きな値・inf・NaN) は std のまま。
結果が 0 のときは x の符号を写す (std と同じく floor(-0.0) = -0.0、ceil(-0.5) = -0.0、round(-0.4) = -0.0)。"""
import re, sys
from pathlib import Path
R = Path(sys.argv[1])
(R / "src/ffloor.rs").write_text('''//! 移植できる速い floor・ceil・round (analysis/buildopt_20261122 の試作、既定のビルドには入れない)。
//! 対象 CPU を指定しない x86-64 の基本 (SSE2) では `f64::floor` などが命令 1 つ (roundsd、SSE4.1) にならず関数呼び出しになるので、
//! |x| < 2^52 なら 0 の向きの切り捨て (cvttsd2si) から作る。結果は std と同じ (0 の符号・NaN・inf・大きな値を含む)。
pub(crate) trait FFloor {
    fn ffloor(self) -> Self;
    fn fceil(self) -> Self;
    fn fround(self) -> Self;
}
const BIG: f64 = 4503599627370496.0; // 2^52: これ以上の |x| は整数 (か inf・NaN)
impl FFloor for f64 {
    #[inline(always)]
    fn ffloor(self) -> f64 {
        if self.abs() < BIG {
            let t = self as i64 as f64;
            let r = if t > self { t - 1.0 } else { t };
            if r == 0.0 { r.copysign(self) } else { r }
        } else {
            self.floor()
        }
    }
    #[inline(always)]
    fn fceil(self) -> f64 {
        if self.abs() < BIG {
            let t = self as i64 as f64;
            let r = if t < self { t + 1.0 } else { t };
            if r == 0.0 { r.copysign(self) } else { r }
        } else {
            self.ceil()
        }
    }
    #[inline(always)]
    fn fround(self) -> f64 {
        if self.abs() < BIG {
            let t = self as i64 as f64;
            let d = self - t; // |x| < 2^52 では正確
            let r = if d >= 0.5 { t + 1.0 } else if d <= -0.5 { t - 1.0 } else { t };
            if r == 0.0 { r.copysign(self) } else { r }
        } else {
            self.round()
        }
    }
}
impl FFloor for f32 {
    #[inline(always)]
    fn ffloor(self) -> f32 { self.floor() }
    #[inline(always)]
    fn fceil(self) -> f32 { self.ceil() }
    #[inline(always)]
    fn fround(self) -> f32 { self.round() }
}

#[cfg(test)]
mod tests {
    use super::FFloor;
    #[test]
    fn same_as_std() {
        let mut v = vec![0.0, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999999999999994, -0.49999999999999994, 1e-300, -1e-300,
            4503599627370495.5, -4503599627370495.5, 4503599627370496.0, -4503599627370497.0, 1e300, -1e300, f64::INFINITY, f64::NEG_INFINITY,
            f64::MIN_POSITIVE, -f64::MIN_POSITIVE, 9.223372036854776e18, -9.223372036854776e18, 3.0, -3.0, 0.9999999999999999, -0.9999999999999999];
        let mut s: u64 = 0x9e3779b97f4a7c15;
        for _ in 0..200000 {
            s ^= s << 13; s ^= s >> 7; s ^= s << 17;
            let e = (s >> 52) as i32 % 80 - 40;
            let m = (s & 0xfffff) as f64 / 1048576.0 - 0.5;
            v.push(m * 2f64.powi(e));
            v.push(f64::from_bits(s));
        }
        for &x in &v {
            for (a, b) in [(x.ffloor(), x.floor()), (x.fceil(), x.ceil()), (x.fround(), x.round())] {
                assert!(a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()), "{x:e}: {a:e} vs {b:e}");
            }
        }
        assert!(f64::NAN.ffloor().is_nan() && f64::NAN.fceil().is_nan() && f64::NAN.fround().is_nan());
    }
}
''')
lib = (R / "src/lib.rs").read_text()
if "mod ffloor;" not in lib:
    lib = lib.replace("mod cancel;\n", "mod cancel;\nmod ffloor;\n", 1)
    (R / "src/lib.rs").write_text(lib)
pat = re.compile(r"\.(floor|ceil|round)\(\)")
n = 0
for f in sorted((R / "src").rglob("*.rs")):
    if f.name == "ffloor.rs": continue
    s = f.read_text()
    s2, k = pat.subn(lambda m: f".f{m.group(1)}()", s)
    if not k: continue
    n += k
    lines = s2.split("\n"); i = 0
    while i < len(lines) and (lines[i].startswith("//!") or lines[i].strip() == "" or lines[i].startswith("#![")):
        if lines[i].startswith("#![") and not lines[i].rstrip().endswith("]"):
            while not lines[i].rstrip().endswith("]"): i += 1
        i += 1
    lines.insert(i, "#[allow(unused_imports)]\nuse crate::ffloor::FFloor as _;")
    f.write_text("\n".join(lines))
print("replaced", n)
