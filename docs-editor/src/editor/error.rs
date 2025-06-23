use std::{fmt, sync::mpsc::SendError};

use crate::editor::EditorAction;

//LeftEditor: salir del editor.
#[derive(Debug)]
pub enum Error {
    LeftEditor,
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::LeftEditor => write!(f, ""),
        }
    }
}

impl From<SendError<EditorAction>> for Error {
    fn from(_: SendError<EditorAction>) -> Self {
        Self::LeftEditor
    }
}
