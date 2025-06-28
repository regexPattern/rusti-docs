use std::{fmt, io, ops::Range};

#[derive(Debug)]
pub enum Error {
    OpenConn(io::Error),
    SendCommand(io::Error),
    ReadReply(io::Error),
    ReplyRespRead(redis_resp::Error),
    ReplyRespType,
    UnsupportedDocType(String),
    MissingData,
    RedisClient(String),
    InvalidContentIndexRange(Range<usize>, usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::OpenConn(err) => write!(f, "error estableciendo conexión con Redis: {err}"),
            Error::SendCommand(err) => write!(f, "error enviando comando a Redis: {err}"),
            Error::ReadReply(err) => write!(f, "error leyendo respuesta de Redis: {err}"),
            Error::ReplyRespRead(err) => {
                write!(f, "error deserializando respuesta de Redis: {err}")
            }
            Error::ReplyRespType => write!(f, "respuesta de Redis tiene tipo inesperado"),
            Error::UnsupportedDocType(basename) => {
                writeln!(f, "tipos de documento {basename} no soportado")?;
                write!(
                    f,
                    "solo se soportan documentos de texto (ext. '.txt') y planillas de cálculo (ext. '.xsl')"
                )
            }
            Error::MissingData => {
                write!(f, "datos faltantes en mensaje de clientes y microservicio")
            }
            Error::RedisClient(err) => write!(f, "error enviado del servidor: {err}"),
            Error::InvalidContentIndexRange(range, n_bytes) => write!(
                f,
                "rango {}-{} es inválido para contenido de {n_bytes} bytes",
                range.start, range.end
            ),
        }
    }
}
impl std::error::Error for Error {}

impl From<redis_resp::Error> for Error {
    fn from(err: redis_resp::Error) -> Self {
        Self::ReplyRespRead(err)
    }
}
