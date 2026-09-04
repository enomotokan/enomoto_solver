//! Compressed Sparse Row (CSR) matrix — the internal storage format for all
//! constraint matrices held inside the Rust core.

/// A row-major sparse matrix in CSR layout: `indptr[i]..indptr[i+1]` indexes
/// into `indices`/`data` for the non-zero entries of row `i`.
#[derive(Debug, Clone)]
pub struct CsrMatrix {
    pub indptr: Vec<usize>,
    pub indices: Vec<usize>,
    pub data: Vec<f64>,
    pub n_rows: usize,
    pub n_cols: usize,
}

impl CsrMatrix {
    /// Build a CSR matrix from a list of sparse rows, each row given as
    /// `(column, value)` pairs. This is the "preprocessing" step that turns
    /// the triplet-style rows accumulated while the problem is being built
    /// into the compact internal representation.
    pub fn from_rows(rows: &[Vec<(usize, f64)>], n_cols: usize) -> Self {
        let mut indptr = Vec::with_capacity(rows.len() + 1);
        let mut indices = Vec::new();
        let mut data = Vec::new();
        indptr.push(0);
        for row in rows {
            let mut sorted = row.clone();
            sorted.sort_by_key(|&(c, _)| c);
            // Merge duplicate column entries (can arise after variable
            // shifting/splitting during standardization).
            let mut merged: Vec<(usize, f64)> = Vec::with_capacity(sorted.len());
            for (c, v) in sorted {
                if let Some(last) = merged.last_mut() {
                    if last.0 == c {
                        last.1 += v;
                        continue;
                    }
                }
                merged.push((c, v));
            }
            for (c, v) in merged {
                if v != 0.0 {
                    indices.push(c);
                    data.push(v);
                }
            }
            indptr.push(indices.len());
        }
        CsrMatrix {
            indptr,
            indices,
            data,
            n_rows: rows.len(),
            n_cols,
        }
    }

    pub fn row(&self, i: usize) -> impl Iterator<Item = (usize, f64)> + '_ {
        let start = self.indptr[i];
        let end = self.indptr[i + 1];
        (start..end).map(move |k| (self.indices[k], self.data[k]))
    }
}
