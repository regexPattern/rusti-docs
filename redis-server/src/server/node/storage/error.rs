use std::{fmt, io, sync::mpsc::SendError};

use log::LogMsg;
use redis_cmd::storage::StorageCommand;
use redis_resp::SimpleError;

/// Errores que pueden ocurrir en el funcionamiento interno del storage actor, no relacionados a las operaciones aplicadas por los comandos.
#[derive(Debug)]
pub enum InternalError {
    LogSend(SendError<LogMsg>),
    PersistenceActorSend(SendError<StorageCommand>),
    PersistenceFileCommand(redis_cmd::Error),
    PersistenceFileFormat(redis_resp::Error),
    PersistenceFileOpen(io::Error),
    PersistenceFileRead(io::Error),
    PersistenceFileWrite(io::Error),
}

impl std::error::Error for InternalError {}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::LogSend(err) => write!(f, "{err}"),
            InternalError::PersistenceActorSend(err) => write!(
                f,
                "error enviando comando ejectuado por el canal del persistence actor: {err}"
            ),
            InternalError::PersistenceFileOpen(err) => {
                write!(f, "error abriendo archivo de persistencia: {err}")
            }
            InternalError::PersistenceFileRead(err) => {
                write!(f, "error leyendo archivo de persistencia: {err}")
            }
            InternalError::PersistenceFileFormat(err) => write!(
                f,
                "contenido de archivo de persistencia no puede ser serializado como comando: {err}"
            ),
            InternalError::PersistenceFileCommand(err) => write!(
                f,
                "archivo de persistencia contiene comandos que no son serializados como comandos de storage: {err}"
            ),
            InternalError::PersistenceFileWrite(err) => {
                write!(f, "error escribiendo al archivo de persistencia: {err}")
            }
        }
    }
}

impl From<SendError<LogMsg>> for InternalError {
    fn from(err: SendError<LogMsg>) -> Self {
        Self::LogSend(err)
    }
}

/// Errores que pueden ocurrir en las operaciones sobre la base de datos.
/// Son errores esperados si el usuario utiliza el comando de manera incorrecta. La situación ante la cual se producen estos errores está documentada en la pagína de cada comando.
#[derive(Debug)]
pub enum OperationError {
    ValueNotAnInteger,
    WrongNumberOfArgs,
    WrongDataType,
}

impl From<OperationError> for SimpleError {
    fn from(err: OperationError) -> Self {
        match err {
            OperationError::ValueNotAnInteger => {
                SimpleError::from("ERROR el valor no es un entero")
            }
            OperationError::WrongNumberOfArgs => {
                SimpleError::from("ERROR número incorrecto de argumentos para el comando")
            }
            OperationError::WrongDataType => SimpleError::from(
                "WRONGTYPE operación contra una clave que contiene un valor del tipo incorrecto",
            ),
        }
    }
}
