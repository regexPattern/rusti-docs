use std::{fmt, io};

/// Enumera los posibles errores de la aplicación relacionados con Redis y documentos.
///
/// - `OpenConn(io::Error)`: Error al abrir una conexión TCP con Redis.
/// - `SendCommand(io::Error)`: Error al enviar un comando a Redis.
/// - `ReadReply(io::Error)`: Error al leer la respuesta de Redis.
/// - `ReplyRespRead(redis_resp::Error)`: Error al deserializar la respuesta de Redis.
/// - `ReplyRespType`: La respuesta de Redis tiene un tipo inesperado.
/// - `UnsupportedDocType(String)`: El tipo de documento no es soportado por la aplicación.
/// - `MissingData`: Faltan datos en el mensaje recibido.
/// - `RedisClient(String)`: Error enviado por el servidor Redis.
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
        }
    }
}
impl std::error::Error for Error {}

impl From<redis_resp::Error> for Error {
    fn from(err: redis_resp::Error) -> Self {
        Self::ReplyRespRead(err)
    }
}
