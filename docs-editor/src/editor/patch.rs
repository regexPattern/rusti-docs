#[derive(Clone, PartialEq, Debug)]
pub enum TextPatch {
    Insert { index: usize, lines: Vec<String> },
    Delete { index: usize, count: usize },
}

pub fn diff_lines(old: &str, new: &str) -> Vec<TextPatch> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    let n = a.len();
    let m = b.len();

    let mut dp = vec![vec![0; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            i += 1;
            j += 1;
        } else if j < m && (i == n || dp[i][j + 1] >= dp[i + 1][j]) {
            let idx = i;
            let mut lines = Vec::new();
            while j < m && (i == n || dp[i][j + 1] >= dp[i + 1][j]) {
                lines.push(b[j].to_string());
                j += 1;
            }
            ops.push(TextPatch::Insert { index: idx, lines });
        } else {
            let idx = i;
            let mut count = 0;
            while i < n && (j == m || dp[i][j + 1] < dp[i + 1][j]) {
                count += 1;
                i += 1;
            }
            ops.push(TextPatch::Delete { index: idx, count });
        }
    }

    ops
}

pub fn apply_text_patches(original: &str, patches: &[TextPatch]) -> String {
    let mut lines: Vec<String> = original.lines().map(String::from).collect();

    let mut ops = patches.to_vec();
    ops.sort_by_key(|op| match op {
        TextPatch::Insert { index, .. } => (*index, 0),
        TextPatch::Delete { index, .. } => (*index, 1),
    });

    let mut offset: isize = 0;
    for op in ops {
        let idx = (match op {
            TextPatch::Insert { index, lines: _ } => index,
            TextPatch::Delete { index, count: _ } => index,
        } as isize
            + offset) as usize;
        match op {
            TextPatch::Insert {
                lines: new_lines, ..
            } => {
                let length = new_lines.len();
                for (k, ln) in new_lines.into_iter().enumerate() {
                    lines.insert(idx + k, ln);
                }
                offset += length as isize;
            }
            TextPatch::Delete { count, .. } => {
                for _ in 0..count {
                    if idx < lines.len() {
                        lines.remove(idx);
                    }
                }
                offset -= count as isize;
            }
        }
    }

    lines.join("\n")
}

#[derive(Clone, PartialEq, Debug)]
pub struct CellPatch<'v> {
    pub row: usize,
    pub col: usize,
    pub value: &'v str,
}

pub fn diff_cells<'v>(
    old: &'v [[String; 10]; 10],
    new: &'v [[String; 10]; 10],
) -> Vec<CellPatch<'v>> {
    let mut patches = Vec::new();

    for i in 0..10 {
        for j in 0..10 {
            if old[i][j] != new[i][j] {
                patches.push(CellPatch {
                    row: i,
                    col: j,
                    value: &new[i][j],
                });
            }
        }
    }

    patches
}

pub fn apply_cell_patches<'v>(
    original: &[[String; 10]; 10],
    patches: &[CellPatch<'v>],
) -> [[String; 10]; 10] {
    let mut result = original.clone();
    for p in patches {
        if p.row < 10 && p.col < 10 {
            result[p.row][p.col] = p.value.to_string();
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_aplican_cambios_al_final_del_archivo() {
        let old = "linea_1\nlinea_2\nlinea_3";
        let new = "linea_1\nlinea_2\nlinea_3\nNUEVA_LINEA";

        let patches = diff_lines(old, new);

        assert_eq!(apply_text_patches(old, &patches), new);
    }

    #[test]
    fn se_aplican_cambios_en_medio_del_archivo() {
        let old = "linea_1\nlinea_2\nlinea_3";
        let new = "linea_1\nNUEVA_LINEA\nlinea_2\nlinea_3\nnueva_linea";

        let patches = diff_lines(old, new);

        assert_eq!(apply_text_patches(old, &patches), new);
    }

    #[test]
    fn se_aplican_cambios_a_lineas_existentes() {
        let old = "linea_1\nlinea_2\nlinea_3";
        let new = "linea_1\nNUEVA_LINEA\nlinea_3";

        let patches = diff_lines(old, new);

        assert_eq!(apply_text_patches(old, &patches), new);
    }

    #[test]
    fn se_aplican_cambios_en_celdas() {
        let mut old: [[String; 10]; 10] = Default::default();
        for i in 0..10 {
            for j in 0..10 {
                old[i][j] = "0".to_string();
            }
        }

        let mut new = old.clone();

        new[0][0] = "1".to_string();
        new[3][7] = "=A1+A2".to_string();
        new[9][9] = "".to_string();

        let patches = diff_cells(&old, &new);

        assert_eq!(patches.len(), 3);

        assert_eq!(
            patches[0],
            CellPatch {
                row: 0,
                col: 0,
                value: "1"
            }
        );
        assert_eq!(
            patches[1],
            CellPatch {
                row: 3,
                col: 7,
                value: "=A1+A2"
            }
        );
        assert_eq!(
            patches[2],
            CellPatch {
                row: 9,
                col: 9,
                value: ""
            }
        );

        assert_eq!(apply_cell_patches(&old, &patches), new);
    }
}
