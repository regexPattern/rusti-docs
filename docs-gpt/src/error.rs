use std::{env::VarError, io};

#[derive(Debug)]
pub enum Error {
    OpenConn(io::Error),
    SendCommand(io::Error),
    ReadReply(io::Error),
    ReplyRespRead(redis_resp::Error),
    ReplyRespType,
    MissingData,
    InvalidApiKey(VarError),
    RedisClient(String),
    LLMRequest(ureq::Error),
    LLMReponseDeser(serde_json::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::OpenConn(err) => write!(f, "error estableciendo conexión con Redis: {err}"),
            Error::SendCommand(err) => write!(f, "error enviando comando a Redis: {err}"),
            Error::ReadReply(err) => write!(f, "error leyendo respuesta de Redis: {err}"),
            Error::ReplyRespRead(err) => {
                write!(f, "error deserializando respuesta de Redis: {err}")
            }
            Error::ReplyRespType => write!(f, "respuesta de Redis tiene tipo inesperado"),
            Error::MissingData => write!(f, "datos faltantes en mensaje de clientes"),
            Error::InvalidApiKey(err) => {
                write!(
                    f,
                    "error al configurar API key para conectar con LLM: {err}"
                )
            }
            Error::RedisClient(err) => write!(f, "error enviado del servidor: {err}"),
            Error::LLMRequest(err) => write!(f, "error leyendo body de respuesta de LLM: {err}"),
            Error::LLMReponseDeser(err) => {
                write!(f, "error de deserialización de respuesta del LLM: {err}")
            }
        }
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Self::LLMRequest(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::LLMReponseDeser(err)
    }
}
