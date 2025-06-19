use core::fmt;

use chrono::{DateTime, FixedOffset};

#[derive(Clone, Debug)]
pub struct DocMetadata {
    pub id: String,
    pub basename: String,
    pub kind: DocKind,
    pub last_edited: DateTime<FixedOffset>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum DocKind {
    Text,
    SpreadSheet,
}

impl fmt::Display for DocKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocKind::Text => write!(f, "TEXT"),
            DocKind::SpreadSheet => write!(f, "SPREADSHEET"),
        }
    }
}

#[derive(Debug)]
pub enum DocContent {
    Text(String),
    SpreadSheet([[String; 10]; 10]),
}

impl DocKind {
    pub fn from_basename(basename: &str) -> Option<Self> {
        let basename = basename.trim_end();
        if basename.ends_with(".txt") {
            Some(Self::Text)
        } else if basename.ends_with(".xsl") {
            Some(Self::SpreadSheet)
        } else {
            None
        }
    }
}

impl fmt::Display for DocContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocContent::Text(content) => write!(f, "{content}"),
            DocContent::SpreadSheet(content) => {
                let values: Vec<_> = content.iter().flat_map(|row| row.iter()).collect();
                write!(
                    f,
                    "{}",
                    values
                        .iter()
                        .map(|v| v.to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn se_detecta_documento_de_texto_a_partir_de_extension() {
        let basename = "doc.txt";
        assert_eq!(DocKind::from_basename(basename).unwrap(), DocKind::Text);
    }

    #[test]
    fn se_detecta_spread_sheet_a_partir_de_extension() {
        let basename = "doc.xsl";
        assert_eq!(
            DocKind::from_basename(basename).unwrap(),
            DocKind::SpreadSheet
        );
    }

    #[test]
    fn deteccion_de_tipo_de_documento_es_case_sensitive() {
        let basename = "doc.TXT";
        assert!(DocKind::from_basename(basename).is_none());
    }
}
