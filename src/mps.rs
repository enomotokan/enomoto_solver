//! MPS ファイル (Mathematical Programming System 形式) の読み込み。
//!
//! Python の `Model.readModel(path)` (`python/enomoto_solver/model.py`) から
//! `PyModel::read_mps` を通して呼ばれる。ファイルを開く・gzip を展開するのは Python 側で、
//! ここは本文 (文字列) を受け取って [`MpsData`] に組み立てるだけ。
//!
//! 読める形:
//! - 自由形式 (空白区切り)。固定欄形式も、名前に空白を含まなければそのまま読める
//!   (MIPLIB・Mittelmann の問題はこの形)。自由形式で読めないとき (名前の重複・欄の数の不一致など) だけ、
//!   データの行を固定欄の桁 (2〜3・5〜12・15〜22・25〜36・40〜47・50〜61 桁) で読み直す
//!   (名前に空白を含む固定欄形式。Netlib の forplan、analysis/forplan_20261123。HiGHS も自由形式の `HMpsFF` で
//!   読めなければ固定形式の `HMPSIO` で読み直す)。読み直しでも読めなければ、自由形式のエラーを返す。
//! - 区切り: `NAME`・`OBJSENSE` (`MAX`/`MIN` を次の行か同じ行に)・`OBJSENSE` の別名
//!   `OBJSENS`・`ROWS`・`COLUMNS` (`'MARKER'` の `'INTORG'`/`'INTEND'` で整数列)・`RHS`・
//!   `RANGES`・`BOUNDS` (`UP`/`LO`/`FX`/`FR`/`MI`/`PL`/`BV`/`LI`/`UI`)・`ENDATA`。
//!   `*` で始まる行は注釈。
//!
//! 約束事は HiGHS の読み込み (`HMpsFF`) にそろえる (計測の台本は HiGHS で読んだ問題を
//! 解いてきたので、同じ問題になるように):
//! - 最初の `N` 行が目的。2 本目以降の `N` 行 (自由行) は捨てる。
//! - 目的の行の右辺 `v` は目的の定数 `-v`。
//! - `MARKER` で囲まれた整数列は、`BOUNDS` に何も書かれなければ `[0, 1]` (二値)。
//!   `BOUNDS` に何か書かれたらその値 (書かれなかった側は既定の `0`・`+inf`)。
//! - `RANGES`: `E` 行は値の符号で上下どちらに広げるかを決め、`L`/`G` 行は絶対値で広げる。
//! - `BOUNDS` の重複した指定 (同じ列の同じ側を 2 回) は後のものを捨てる。
//! - `UP` で負の値を与え、下限が書かれていなければ下限を `-inf` にする (多くの読み手の約束。
//!   HiGHS は下限 0 のまま)。
//!
//! 半連続 (`SC`)・半整数 (`SI`)・二次の項 (`QUADOBJ` など)・`SOS` は扱えないので、
//! 見つけたらエラーにする。`ROWS` の節が無いファイル (空・注釈だけ・`NAME` だけなど) もエラーにする
//! (変数 0 個の問題として黙って読まない)。

use std::collections::HashMap;

/// 行の種類 (`ROWS` の `N`/`L`/`G`/`E`)。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RowKind {
    /// `<=`
    Le,
    /// `>=`
    Ge,
    /// `=`
    Eq,
}

/// 読み込んだ問題。行は `lo <= a·x <= hi` の範囲の形 (片側は `±inf`)。
#[derive(Debug, Default)]
pub struct MpsData {
    /// 問題の名前 (`NAME` の行。なければ空)。
    pub name: String,
    /// 列の名前 (列番号の順)。
    pub col_names: Vec<String>,
    /// 列の下限・上限。
    pub col_lb: Vec<f64>,
    pub col_ub: Vec<f64>,
    /// 整数列か。
    pub is_int: Vec<bool>,
    /// 目的の係数 (列ごと)。
    pub cost: Vec<f64>,
    /// 目的の定数。
    pub offset: f64,
    /// 最大化か (`OBJSENSE MAX`)。
    pub maximize: bool,
    /// 行の名前 (行番号の順。目的の行と捨てた自由行は含まない)。
    pub row_names: Vec<String>,
    /// 行ごとの非零 (列番号, 係数)。
    pub rows: Vec<Vec<(usize, f64)>>,
    /// 行の下限・上限。
    pub row_lo: Vec<f64>,
    pub row_hi: Vec<f64>,
    /// 読み飛ばした箇所の注意 (定義されていない行・列への参照。HiGHS と同じく警告して飛ばす)。
    pub warnings: Vec<String>,
}

/// 区切り。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Section {
    None,
    Name,
    ObjSense,
    Rows,
    Columns,
    Rhs,
    Ranges,
    Bounds,
    End,
}

/// 数の読み取り (失敗・NaN は行番号つきのエラー)。
fn num(tok: &str, line_no: usize) -> Result<f64, String> {
    // Fortran の指数表記 `1.0D3` も読む
    let parsed = if tok.contains(['d', 'D']) { tok.replace(['d', 'D'], "e").parse::<f64>() } else { tok.parse::<f64>() };
    match parsed {
        Ok(v) if !v.is_nan() => Ok(v),
        _ => Err(format!("line {line_no}: cannot read the number {tok:?}")),
    }
}

/// 行の名前の参照先: 目的の行・捨てた自由行・通常の行 (番号)。
#[derive(Clone, Copy)]
enum RowRef {
    Objective,
    Free,
    Row(usize),
}

/// MPS の本文を読む。読めなければ理由を返す。自由形式で読めなければ固定欄形式で読み直す (モジュールの説明)。
pub fn parse_mps(text: &str) -> Result<MpsData, String> {
    match parse_mps_with(text, false) {
        Ok(d) => Ok(d),
        Err(e) => parse_mps_with(text, true).map_err(|_| e),
    }
}

/// 固定欄形式のデータの行の欄 (1 始まりの桁 2〜3・5〜12・15〜22・25〜36・40〜47・50〜61) を、前後の空白を除いて返す。
/// 空の欄は落とす (自由形式の空白区切りと同じ並びになる。RHS・RANGES の名前の欄が空なら、自由形式の名前なしと同じ)。
/// 最後の欄は 61 桁で切らず行の終わりまで取る (数値が長く書かれた行のため)。
fn fixed_fields(raw: &str) -> Vec<String> {
    const F: [(usize, usize); 6] = [(1, 3), (4, 12), (14, 22), (24, 36), (39, 47), (49, usize::MAX)];
    let b = raw.as_bytes();
    let mut out = Vec::new();
    for &(a, e) in &F {
        if a >= b.len() {
            break;
        }
        let e = e.min(b.len());
        let f = String::from_utf8_lossy(&b[a..e]).trim().to_string();
        if !f.is_empty() {
            out.push(f);
        }
    }
    out
}

/// [`parse_mps`] の本体。`fixed` なら区切りの行でないデータの行を固定欄の桁で読む ([`fixed_fields`])
fn parse_mps_with(text: &str, fixed: bool) -> Result<MpsData, String> {
    let mut d = MpsData::default();
    let mut sec = Section::None;
    let mut row_kind: Vec<RowKind> = Vec::new();
    let mut row_of: HashMap<String, RowRef> = HashMap::new();
    let mut col_of: HashMap<String, usize> = HashMap::new();
    let mut objective_name: Option<String> = None;
    // `ROWS` の節が一度でも現れたか (読めるデータが無いファイルを見分けるため)
    let mut saw_rows = false;
    let mut in_int = false;
    // 列ごとの「BOUNDS で下限・上限が書かれたか」と「MARKER の整数列で、まだ既定の二値のままか」
    let mut has_lb: Vec<bool> = Vec::new();
    let mut has_ub: Vec<bool> = Vec::new();
    let mut binary_default: Vec<bool> = Vec::new();
    // RANGES で同じ行を 2 回指定したら後のものを捨てる
    let mut ranged: Vec<bool> = Vec::new();
    // 列の非零 (列, 行) の重複は合算する (HiGHS も同じ列・行の 2 回目を足す)
    let mut entries: Vec<HashMap<usize, f64>> = Vec::new();

    for (k, raw) in text.lines().enumerate() {
        let line_no = k + 1;
        let line = raw.trim_end();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        let starts_at_col0 = !line.starts_with(|c: char| c.is_whitespace());
        let owned: Vec<String> = if fixed && !starts_at_col0 { fixed_fields(line) } else { Vec::new() };
        let toks: Vec<&str> = if fixed && !starts_at_col0 { owned.iter().map(|s| s.as_str()).collect() } else { line.split_whitespace().collect() };
        if toks.is_empty() {
            continue;
        }
        if starts_at_col0 {
            let key = toks[0].to_ascii_uppercase();
            let next = match key.as_str() {
                "NAME" => Some(Section::Name),
                "OBJSENSE" | "OBJSENS" => Some(Section::ObjSense),
                "ROWS" => Some(Section::Rows),
                "COLUMNS" => Some(Section::Columns),
                "RHS" => Some(Section::Rhs),
                "RANGES" => Some(Section::Ranges),
                "BOUNDS" => Some(Section::Bounds),
                "ENDATA" => Some(Section::End),
                "SOS" | "QUADOBJ" | "QSECTION" | "QMATRIX" | "QCMATRIX" | "INDICATORS" | "GENCONS" | "PWLOBJ" => {
                    return Err(format!("line {line_no}: the section {key} is not supported (only linear problems with continuous and integer columns)"));
                }
                _ => None,
            };
            if let Some(s) = next {
                sec = s;
                if s == Section::Rows {
                    saw_rows = true;
                }
                match s {
                    Section::Name => d.name = toks.get(1).map(|s| s.to_string()).unwrap_or_default(),
                    // `OBJSENSE MAX` を同じ行に書く形 (Gurobi など)
                    Section::ObjSense => {
                        if let Some(v) = toks.get(1) {
                            d.maximize = parse_sense(v, line_no)?;
                        }
                    }
                    Section::End => break,
                    _ => {}
                }
                continue;
            }
            // 区切りの名前でない行が 0 桁目から始まっていても、自由形式ではデータの行として読む
        }
        match sec {
            Section::None | Section::Name | Section::End => {
                return Err(format!("line {line_no}: data line outside of a section: {line:?}"));
            }
            Section::ObjSense => {
                d.maximize = parse_sense(toks[0], line_no)?;
            }
            Section::Rows => {
                if toks.len() < 2 {
                    return Err(format!("line {line_no}: a ROWS line needs a type and a name"));
                }
                let name = toks[1].to_string();
                if row_of.contains_key(&name) {
                    return Err(format!("line {line_no}: the row {name:?} is defined twice"));
                }
                let kind = match toks[0].to_ascii_uppercase().as_str() {
                    "N" => {
                        if objective_name.is_none() {
                            objective_name = Some(name.clone());
                            row_of.insert(name, RowRef::Objective);
                        } else {
                            row_of.insert(name, RowRef::Free);
                        }
                        continue;
                    }
                    "L" => RowKind::Le,
                    "G" => RowKind::Ge,
                    "E" => RowKind::Eq,
                    t => return Err(format!("line {line_no}: unknown row type {t:?}")),
                };
                let i = d.row_names.len();
                row_of.insert(name.clone(), RowRef::Row(i));
                d.row_names.push(name);
                row_kind.push(kind);
                // 右辺の既定は 0
                let (lo, hi) = match kind {
                    RowKind::Le => (f64::NEG_INFINITY, 0.0),
                    RowKind::Ge => (0.0, f64::INFINITY),
                    RowKind::Eq => (0.0, 0.0),
                };
                d.row_lo.push(lo);
                d.row_hi.push(hi);
                ranged.push(false);
            }
            Section::Columns => {
                // 整数の印: `名前 'MARKER' 'INTORG'` / `'INTEND'`
                if toks.len() >= 3 && toks[1].trim_matches('\'').eq_ignore_ascii_case("MARKER") {
                    let m = toks[2].trim_matches('\'').to_ascii_uppercase();
                    match m.as_str() {
                        "INTORG" => in_int = true,
                        "INTEND" => in_int = false,
                        _ => return Err(format!("line {line_no}: unknown marker {m:?}")),
                    }
                    continue;
                }
                if toks.len() != 3 && toks.len() != 5 {
                    return Err(format!("line {line_no}: a COLUMNS line needs a column name and one or two (row, value) pairs"));
                }
                let cname = toks[0];
                let j = match col_of.get(cname) {
                    Some(&j) => j,
                    None => {
                        let j = d.col_names.len();
                        col_of.insert(cname.to_string(), j);
                        d.col_names.push(cname.to_string());
                        d.cost.push(0.0);
                        d.is_int.push(in_int);
                        d.col_lb.push(0.0);
                        d.col_ub.push(f64::INFINITY);
                        has_lb.push(false);
                        has_ub.push(false);
                        binary_default.push(in_int);
                        entries.push(HashMap::new());
                        j
                    }
                };
                for p in toks[1..].chunks(2) {
                    let v = num(p[1], line_no)?;
                    match row_of.get(p[0]) {
                        Some(RowRef::Objective) => d.cost[j] += v,
                        Some(RowRef::Free) => {}
                        Some(RowRef::Row(i)) => *entries[j].entry(*i).or_insert(0.0) += v,
                        None => d.warnings.push(format!("line {line_no}: the row {:?} is not defined in ROWS; the entry is ignored", p[0])),
                    }
                }
            }
            Section::Rhs | Section::Ranges => {
                // `[名前] 行 値 [行 値]`: 数が偶数なら名前なし、奇数なら先頭が名前
                let pairs = if toks.len() % 2 == 0 { &toks[..] } else { &toks[1..] };
                if pairs.is_empty() || pairs.len() > 4 {
                    return Err(format!("line {line_no}: cannot read the line {line:?}"));
                }
                for p in pairs.chunks(2) {
                    let v = num(p[1], line_no)?;
                    let r = match row_of.get(p[0]) {
                        Some(r) => *r,
                        None => {
                            d.warnings.push(format!("line {line_no}: the row {:?} is not defined in ROWS; the entry is ignored", p[0]));
                            continue;
                        }
                    };
                    if sec == Section::Rhs {
                        match r {
                            RowRef::Objective => d.offset = -v,
                            RowRef::Free => {}
                            RowRef::Row(i) => match row_kind[i] {
                                RowKind::Le => d.row_hi[i] = v,
                                RowKind::Ge => d.row_lo[i] = v,
                                RowKind::Eq => {
                                    d.row_lo[i] = v;
                                    d.row_hi[i] = v;
                                }
                            },
                        }
                    } else {
                        // RANGES は RHS の後に書かれる約束。目的の行・自由行への範囲は捨てる
                        if let RowRef::Row(i) = r {
                            if ranged[i] {
                                continue;
                            }
                            ranged[i] = true;
                            apply_range(&mut d, row_kind[i], i, v);
                        }
                    }
                }
            }
            Section::Bounds => {
                parse_bound(&toks, line_no, &mut d, &col_of, &mut has_lb, &mut has_ub, &mut binary_default)?;
            }
        }
    }

    // 読めるデータが無いファイル (空・注釈だけ・NAME だけなど) はエラーにする。`ENDATA` が無いだけのファイルは
    // ここまでに ROWS と中身を読めているので、今までどおり読める
    if !saw_rows || (objective_name.is_none() && d.row_names.is_empty() && d.col_names.is_empty()) {
        return Err("no MPS data found (the file is empty or has no ROWS section)".to_string());
    }
    // MARKER の整数列で BOUNDS に何も書かれなかったものは二値
    for j in 0..d.col_names.len() {
        if binary_default[j] {
            d.col_lb[j] = 0.0;
            d.col_ub[j] = 1.0;
        }
    }
    // 行の非零を行方向に並べ直す (列番号の昇順)
    d.rows = vec![Vec::new(); d.row_names.len()];
    for (j, e) in entries.iter().enumerate() {
        for (&i, &v) in e {
            if v != 0.0 {
                d.rows[i].push((j, v));
            }
        }
    }
    for r in &mut d.rows {
        r.sort_unstable_by_key(|&(j, _)| j);
    }
    Ok(d)
}

/// `OBJSENSE` の値 (`MAX`/`MAXIMIZE`/`MIN`/`MINIMIZE`)。
fn parse_sense(tok: &str, line_no: usize) -> Result<bool, String> {
    match tok.to_ascii_uppercase().as_str() {
        "MAX" | "MAXIMIZE" | "MAXIMISE" => Ok(true),
        "MIN" | "MINIMIZE" | "MINIMISE" => Ok(false),
        t => Err(format!("line {line_no}: unknown OBJSENSE {t:?}")),
    }
}

/// `RANGES` の値 `r` を行 `i` に当てる。`E` 行は符号で上下どちらに広げるかを決め、
/// `L` 行は下へ、`G` 行は上へ `|r|` だけ広げる。
fn apply_range(d: &mut MpsData, kind: RowKind, i: usize, r: f64) {
    match kind {
        RowKind::Le => d.row_lo[i] = d.row_hi[i] - r.abs(),
        RowKind::Ge => d.row_hi[i] = d.row_lo[i] + r.abs(),
        RowKind::Eq => {
            if r > 0.0 {
                d.row_hi[i] = d.row_lo[i] + r;
            } else if r < 0.0 {
                d.row_lo[i] = d.row_hi[i] + r;
            }
        }
    }
}

/// `BOUNDS` の 1 行: `型 [名前] 列 [値]`。
#[allow(clippy::too_many_arguments)]
fn parse_bound(
    toks: &[&str],
    line_no: usize,
    d: &mut MpsData,
    col_of: &HashMap<String, usize>,
    has_lb: &mut [bool],
    has_ub: &mut [bool],
    binary_default: &mut [bool],
) -> Result<(), String> {
    let ty = toks[0].to_ascii_uppercase();
    let needs_value = matches!(ty.as_str(), "UP" | "LO" | "FX" | "LI" | "UI");
    let known = needs_value || matches!(ty.as_str(), "FR" | "MI" | "PL" | "BV");
    if !known {
        return Err(format!("line {line_no}: the bound type {ty:?} is not supported"));
    }
    // 名前 (束の名前) は省いてもよい。先頭の次が列の名前なら省かれている
    let rest = &toks[1..];
    let (col, val) = if rest.first().is_some_and(|t| col_of.contains_key(*t)) && (if needs_value { rest.len() == 2 } else { rest.len() <= 2 }) {
        (rest[0], rest.get(1).copied())
    } else if rest.len() >= 2 {
        (rest[1], rest.get(2).copied())
    } else {
        return Err(format!("line {line_no}: cannot read the bound line"));
    };
    let j = match col_of.get(col) {
        Some(&j) => j,
        None => {
            d.warnings.push(format!("line {line_no}: the column {col:?} is not defined in COLUMNS; the bound is ignored"));
            return Ok(());
        }
    };
    let value = if needs_value {
        match val {
            Some(v) => Some(num(v, line_no)?),
            None => return Err(format!("line {line_no}: the bound {ty} needs a value")),
        }
    } else {
        None
    };
    let (sets_lb, sets_ub) = match ty.as_str() {
        "UP" | "UI" | "PL" => (false, true),
        "LO" | "LI" | "MI" => (true, false),
        _ => (true, true), // FX・FR・BV
    };
    // 同じ側の重複は後のものを捨てる
    if (sets_lb && has_lb[j]) || (sets_ub && has_ub[j]) {
        return Ok(());
    }
    match ty.as_str() {
        "BV" => {
            d.is_int[j] = true;
            d.col_lb[j] = 0.0;
            d.col_ub[j] = 1.0;
            binary_default[j] = true;
        }
        "FR" => {
            d.col_lb[j] = f64::NEG_INFINITY;
            d.col_ub[j] = f64::INFINITY;
            binary_default[j] = false;
        }
        "MI" => {
            d.col_lb[j] = f64::NEG_INFINITY;
            binary_default[j] = false;
        }
        "PL" => {
            d.col_ub[j] = f64::INFINITY;
            binary_default[j] = false;
        }
        _ => {
            let v = value.unwrap();
            if ty == "LI" || ty == "UI" {
                d.is_int[j] = true;
            }
            if binary_default[j] {
                // 既定の二値から外れる: 書かれなかった側は 0・+inf
                binary_default[j] = false;
                d.col_lb[j] = 0.0;
                d.col_ub[j] = f64::INFINITY;
            }
            match ty.as_str() {
                "UP" | "UI" => {
                    d.col_ub[j] = v;
                    if v < 0.0 && !has_lb[j] && d.col_lb[j] == 0.0 {
                        d.col_lb[j] = f64::NEG_INFINITY;
                    }
                }
                "LO" | "LI" => d.col_lb[j] = v,
                _ => {
                    d.col_lb[j] = v;
                    d.col_ub[j] = v;
                }
            }
        }
    }
    if sets_lb {
        has_lb[j] = true;
    }
    if sets_ub {
        has_ub[j] = true;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SMALL: &str = "\
NAME          TESTLP
* comment
OBJSENSE
    MAX
ROWS
 N  COST
 L  LIM1
 G  LIM2
 E  MYEQN
 L  R4
COLUMNS
    X1        COST         1.0   LIM1         1.0
    X1        LIM2         1.0
    MARKER    'MARKER'     'INTORG'
    Y         COST         2.0   LIM1         1.0
    Y         MYEQN       -1.0
    Z         COST        -1.0   MYEQN        1.0
    MARKER    'MARKER'     'INTEND'
    W         R4           3.0
RHS
    RHS       COST        -5.0
    RHS       LIM1         4.0   LIM2         1.0
    RHS       MYEQN        7.0   R4           9.0
RANGES
    RNG       LIM1         2.5   MYEQN       -2.0
    RNG       R4           1.0
BOUNDS
 UP BND       X1           4.0
 MI BND       W
 LO BND       Z            -3
ENDATA
";

    /// 読めるデータが無いファイル (空・注釈だけ・`NAME` だけ) はエラー: 変数 0 個の問題として黙って読まない
    #[test]
    fn rejects_files_without_rows() {
        for t in ["", "\n\n", "* comment\n\n", "NAME x\n", "NAME x\nENDATA\n"] {
            let e = parse_mps(t).unwrap_err();
            assert!(e.contains("no MPS data found"), "{t:?} -> {e}");
        }
        // `ENDATA` が無いだけのファイルは今までどおり読める
        let d = parse_mps("NAME x\nROWS\n N obj\n L c\nCOLUMNS\n a obj 1 c 1\nRHS\n rhs c 2\n").unwrap();
        assert_eq!((d.row_names.clone(), d.col_names.clone()), (vec!["c".to_string()], vec!["a".to_string()]));
    }

    #[test]
    fn reads_small_problem() {
        let d = parse_mps(SMALL).unwrap();
        assert_eq!(d.name, "TESTLP");
        assert!(d.maximize);
        assert_eq!(d.col_names, vec!["X1", "Y", "Z", "W"]);
        assert_eq!(d.cost, vec![1.0, 2.0, -1.0, 0.0]);
        assert_eq!(d.offset, 5.0);
        assert_eq!(d.is_int, vec![false, true, true, false]);
        // X1: UP 4。Y: 整数で BOUNDS なし → 二値。Z: 整数で LO -3 → [-3, inf)。W: MI → (-inf, inf)
        assert_eq!((d.col_lb[0], d.col_ub[0]), (0.0, 4.0));
        assert_eq!((d.col_lb[1], d.col_ub[1]), (0.0, 1.0));
        assert_eq!((d.col_lb[2], d.col_ub[2]), (-3.0, f64::INFINITY));
        assert_eq!((d.col_lb[3], d.col_ub[3]), (f64::NEG_INFINITY, f64::INFINITY));
        assert_eq!(d.row_names, vec!["LIM1", "LIM2", "MYEQN", "R4"]);
        // LIM1: L 4 に範囲 2.5 → [1.5, 4]。LIM2: G 1。MYEQN: E 7 に範囲 -2 → [5, 7]。R4: L 9 範囲 1 → [8, 9]
        assert_eq!((d.row_lo[0], d.row_hi[0]), (1.5, 4.0));
        assert_eq!((d.row_lo[1], d.row_hi[1]), (1.0, f64::INFINITY));
        assert_eq!((d.row_lo[2], d.row_hi[2]), (5.0, 7.0));
        assert_eq!((d.row_lo[3], d.row_hi[3]), (8.0, 9.0));
        assert_eq!(d.rows[0], vec![(0, 1.0), (1, 1.0)]);
        assert_eq!(d.rows[2], vec![(1, -1.0), (2, 1.0)]);
        assert_eq!(d.rows[3], vec![(3, 3.0)]);
    }

    /// 名前に空白を含む固定欄形式 (Netlib の forplan の形、analysis/forplan_20261123): 自由形式では行の名前が重なって読めないので、固定欄の桁で読み直す
    #[test]
    fn fixed_format_with_spaces_in_names() {
        let t = "NAME          FIXSP\nROWS\n N  OB J\n E  DEDO3 1R\n E  DEDO3 2R\n L  CAP A\nCOLUMNS\n    X 1       OB J                1.   DEDO3 1R            1.\n    X 1       CAP A               1.\n    X 2       OB J                2.   DEDO3 2R            1.\n    X 2       CAP A               1.\nRHS\n    RHS       DEDO3 1R            3.   DEDO3 2R            4.\n    RHS       CAP A              10.\nRANGES\n    RNG       CAP A               2.\nBOUNDS\n UP BND       X 1                 5.\nENDATA\n";
        let d = parse_mps(t).unwrap();
        assert_eq!(d.row_names, vec!["DEDO3 1R", "DEDO3 2R", "CAP A"]);
        assert_eq!(d.col_names, vec!["X 1", "X 2"]);
        assert_eq!(d.cost, vec![1.0, 2.0]);
        assert_eq!((d.row_lo.clone(), d.row_hi.clone()), (vec![3.0, 4.0, 8.0], vec![3.0, 4.0, 10.0]));
        assert_eq!(d.rows, vec![vec![(0, 1.0)], vec![(1, 1.0)], vec![(0, 1.0), (1, 1.0)]]);
        assert_eq!((d.col_lb.clone(), d.col_ub.clone()), (vec![0.0, 0.0], vec![5.0, f64::INFINITY]));
        // 自由形式で読める本文は固定欄で読み直さない (名前の前後の空白で区切る今までの読み方のまま)
        let f = "NAME x\nROWS\n N obj\n L c\nCOLUMNS\n a obj 1 c 1\nRHS\n rhs c 2\nENDATA\n";
        assert_eq!(parse_mps(f).unwrap().row_names, vec!["c"]);
    }

    #[test]
    fn objsense_on_same_line_and_free_rows() {
        let t = "NAME x\nOBJSENSE MAXIMIZE\nROWS\n N obj\n N free\n G c\nCOLUMNS\n a obj 1 free 9\n a c 1\nRHS\n c 2\nBOUNDS\n BV b a\nENDATA\n";
        let d = parse_mps(t).unwrap();
        assert!(d.maximize);
        assert_eq!(d.row_names, vec!["c"]);
        assert_eq!(d.rows[0], vec![(0, 1.0)]);
        assert_eq!((d.row_lo[0], d.row_hi[0]), (2.0, f64::INFINITY));
        assert!(d.is_int[0]);
        assert_eq!((d.col_lb[0], d.col_ub[0]), (0.0, 1.0));
    }

    #[test]
    fn negative_upper_bound_frees_lower() {
        let t = "NAME\nROWS\n N obj\nCOLUMNS\n a obj 1\nBOUNDS\n UP B a -2\nENDATA\n";
        let d = parse_mps(t).unwrap();
        assert_eq!((d.col_lb[0], d.col_ub[0]), (f64::NEG_INFINITY, -2.0));
    }

    #[test]
    fn fortran_exponent_and_undefined_names() {
        let t = "NAME\nROWS\n N obj\n G c\nCOLUMNS\n a obj -1 c 1\n a zz 2\nRHS\n R zz 3\nBOUNDS\n UP B a 1.0D3\n UP B q 1\nENDATA\n";
        let d = parse_mps(t).unwrap();
        assert_eq!(d.col_ub[0], 1000.0);
        assert_eq!(d.rows[0], vec![(0, 1.0)]);
        assert_eq!(d.warnings.len(), 3);
    }

    #[test]
    fn rejects_unsupported() {
        let t = "NAME\nROWS\n N obj\nCOLUMNS\n a obj 1\nBOUNDS\n SC B a 3\nENDATA\n";
        assert!(parse_mps(t).is_err());
        let t = "NAME\nROWS\n N obj\nCOLUMNS\n a obj 1\nQUADOBJ\n a a 1\nENDATA\n";
        assert!(parse_mps(t).is_err());
    }
}
