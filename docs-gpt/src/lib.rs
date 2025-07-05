mod error;

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{Shutdown, SocketAddr, TcpStream},
    sync::mpsc::{self, Sender},
    thread::{self, JoinHandle},
};

use redis_cmd::{
    Command,
    pub_sub::{PubSubCommand, Publish, Subscribe},
    storage::{Get, StorageCommand},
};
use redis_resp::{BulkString, RespDataType};
use serde_json::Value;
use thread_pool::ThreadPool;

use crate::error::Error;

const OPENAI_API_KEY_ENV: &str = "OPENAI_API_KEY";
const OPENAI_API_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const OPENAI_MODEL: &str = "gpt-4.1-nano-2025-04-14";

/// Estructura principal del microservicio DocsGpt.
///
/// Permite gestionar la interacción con la API de LLM y la base de datos Redis.
#[derive(Debug)]
pub struct DocsGpt {
    db_addr: SocketAddr,
    api_key: String,
    thread_pool: ThreadPool,
}

/// Acciones internas que puede ejecutar DocsGpt.
#[derive(Debug)]
enum DocsGptAction {
    /// Solicita generación de texto al LLM.
    PromptLLM {
        kind: EditKind,
        client_id: String,
        doc_id: String,
        user_prompt: String,
        doc_content: String,
    },
}

/// Tipo de edición solicitada al LLM.
///
/// - `Partial`: Edición parcial, devuelve el rango de bytes editado.
/// - `Full`: Edición completa, reemplaza todo el documento.
#[derive(Debug)]
enum EditKind {
    /// Edición parcial del documento.
    Partial,
    /// Edición completa del documento.
    Full,
}

impl DocsGpt {
    /// Crea una nueva instancia de DocsGpt.
    pub fn new(db_addr: SocketAddr) -> Result<Self, Error> {
        let api_key = std::env::var(OPENAI_API_KEY_ENV).map_err(Error::InvalidApiKey)?;

        Ok(Self {
            db_addr,
            api_key,
            thread_pool: ThreadPool::new(8),
        })
    }

    /// Inicia el microservicio y los hilos de procesamiento.
    pub fn start(self) -> Result<Vec<JoinHandle<()>>, Error> {
        let db_addr = self.db_addr;
        let (actions_tx, actions_rx) = mpsc::channel();

        Ok(vec![
            self.handle_actions(actions_rx),
            Self::start_content_generator(db_addr, actions_tx.clone())?,
        ])
    }

    fn handle_actions(self, actions_rx: mpsc::Receiver<DocsGptAction>) -> JoinHandle<()> {
        thread::spawn(move || {
            for action in actions_rx {
                match action {
                    DocsGptAction::PromptLLM {
                        kind,
                        client_id,
                        doc_id,
                        user_prompt,
                        doc_content,
                    } => {
                        print!(
                            "{}",
                            log::info!(
                                "solicitando generación {} de documento {doc_id}",
                                match kind {
                                    EditKind::Partial => "parcial",
                                    EditKind::Full => "completa",
                                }
                            )
                        );

                        self.generate_content(kind, client_id, doc_id, user_prompt, doc_content);
                    }
                }
            }
        })
    }

    fn start_content_generator(
        db_addr: SocketAddr,
        actions_tx: Sender<DocsGptAction>,
    ) -> Result<JoinHandle<()>, Error> {
        let cmd = Command::PubSub(PubSubCommand::Subscribe(Subscribe {
            channels: vec!["docs_gpt".into()],
        }));

        let mut docs_gpt_stream = TcpStream::connect(db_addr).map_err(Error::OpenConn)?;

        docs_gpt_stream
            .write_all(&Vec::from(cmd))
            .map_err(Error::SendCommand)?;

        print!(
            "{}",
            log::info!("iniciando canal de escucha de editores clientes")
        );

        Ok(thread::spawn(move || {
            let mut buffer = BufReader::new(&mut docs_gpt_stream);

            loop {
                match buffer.fill_buf() {
                    Ok(bytes) if !bytes.is_empty() => {
                        Self::handle_documents_prompt_message(bytes, &actions_tx).unwrap();
                        let length = bytes.len();
                        buffer.consume(length);
                    }
                    Ok(_) => {
                        print!(
                            "{}",
                            log::warn!("desconectado stream de solicitud de generaciones")
                        );
                        return;
                    }
                    Err(err) => {
                        print!(
                            "{}",
                            log::error!("error leyendo stream de solicitud de generaciones: {err}")
                        );
                        return;
                    }
                }
            }
        }))
    }

    fn handle_documents_prompt_message(
        bytes: &[u8],
        actions_tx: &Sender<DocsGptAction>,
    ) -> Result<(), Error> {
        let mut payload = match RespDataType::try_from(bytes).map_err(Error::ReplyRespRead)? {
            RespDataType::Array(payload) => payload.into_iter().filter_map(|e| {
                if let RespDataType::BulkString(e) = e {
                    Some(e)
                } else {
                    None
                }
            }),
            _ => unreachable!(),
        };

        if let Some(payload) = payload.nth(2) {
            match payload
                .to_string()
                .split_once('@')
                .ok_or(Error::MissingData)?
            {
                // PARTIALGEN_REQ@<CLIENT_ID>@<DOC_ID>@<USER_PROMPT>@<DOC_CONTENT>
                ("PARTIALGEN_REQ", payload) => {
                    let mut payload = payload.splitn(4, '@').map(String::from);
                    let _ = actions_tx.send(DocsGptAction::PromptLLM {
                        kind: EditKind::Partial,
                        client_id: payload.next().ok_or(Error::MissingData)?,
                        doc_id: payload.next().ok_or(Error::MissingData)?,
                        user_prompt: payload.next().ok_or(Error::MissingData)?,
                        doc_content: payload.next().ok_or(Error::MissingData)?,
                    });
                }
                // FULLGEN_REQ@<CLIENT_ID>@<DOC_ID>@<USER_PROMPT>
                ("FULLGEN_REQ", payload) => {
                    let mut payload = payload.splitn(3, '@').map(String::from);
                    let _ = actions_tx.send(DocsGptAction::PromptLLM {
                        kind: EditKind::Full,
                        client_id: payload.next().ok_or(Error::MissingData)?,
                        doc_id: payload.next().ok_or(Error::MissingData)?,
                        user_prompt: payload.next().ok_or(Error::MissingData)?,
                        doc_content: "".to_string(),
                    });
                }
                _ => {}
            }
        }

        Ok(())
    }

    fn generate_content(
        &self,
        kind: EditKind,
        client_id: String,
        doc_id: String,
        user_prompt: String,
        mut doc_content: String,
    ) {
        let db_addr = self.db_addr;
        let auth_header_val = format!("Bearer {}", self.api_key);

        let _ = self.thread_pool.execute(move || {
            if matches!(kind, EditKind::Full) {
                doc_content = match Self::fetch_document(db_addr, &doc_id) {
                    Ok(doc_content) => doc_content,
                    Err(err) => {
                        print!(
                            "{}",
                            log::error!("error obteniendo contenido de documento {doc_id}: {err}")
                        );
                        return;
                    }
                };
            }

            let gen_content = match Self::prompt_llm(&auth_header_val, &user_prompt, &doc_content) {
                Ok(gen_content) => gen_content,
                Err(err) => {
                    print!("{}", log::error!("error generando contenido de documento {doc_id} para cliente {client_id}: {err}"));
                    return;
                },
            };

            let msg = match kind {
                // PARTIALGEN_ACK@<CLIENT_ID>@<GENERATED_CONTENT>
                EditKind::Partial => format!("PARTIALGEN_ACK@{client_id}@{gen_content}"),
                // FULLGEN_ACK@<CLIENT_ID>@<GENERATED_CONTENT>
                EditKind::Full => format!("FULLGEN_ACK@{client_id}@{gen_content}"),
            };

            let mut stream = TcpStream::connect(db_addr).unwrap();

            let cmd = Vec::from(Command::PubSub(PubSubCommand::Publish(Publish {
                channel: doc_id.into(),
                message: msg.into(),
            })));

            stream.write_all(&cmd).unwrap();

            print!("{}", log::info!("enviada respuesta de LLM a clientes"))
        });
    }

    fn fetch_document(mut slot_addr: SocketAddr, doc_id: &str) -> Result<String, Error> {
        let cmd = Vec::from(Command::Storage(StorageCommand::Get(Get {
            key: BulkString::from(doc_id),
        })));

        loop {
            let mut stream = TcpStream::connect(slot_addr).map_err(Error::OpenConn)?;
            stream.write_all(&cmd).map_err(Error::SendCommand)?;
            stream.shutdown(Shutdown::Write).map_err(Error::OpenConn)?;

            let mut buffer = Vec::new();
            stream.read_to_end(&mut buffer).map_err(Error::ReadReply)?;

            let doc_content =
                RespDataType::try_from(buffer.as_slice()).map_err(|_| Error::ReplyRespType)?;

            let doc_content = match doc_content {
                RespDataType::BulkString(doc_content) => doc_content.to_string(),
                RespDataType::Null => "".to_string(),
                RespDataType::SimpleError(err) => {
                    if err.0.contains("MOVED") {
                        let port = err.0.split(":").last().ok_or(Error::MissingData)?;
                        let port = port.parse().unwrap();
                        print!("{}", log::debug!("redirigiendo a nodo en puerto {port}"));
                        slot_addr.set_port(port);
                        continue;
                    } else {
                        return Err(Error::RedisClient(err.0));
                    }
                }
                _ => unreachable!(),
            };

            print!("{}", log::debug!("comando enviado a nodo en {slot_addr:?}"));

            return Ok(doc_content);
        }
    }

    fn prompt_llm(
        auth_header_val: &str,
        user_prompt: &str,
        doc_content: &str,
    ) -> Result<String, Error> {
        let mut res = ureq::post(OPENAI_API_ENDPOINT)
            .header("Authorization", auth_header_val)
            .send_json(serde_json::json!({
                "model": OPENAI_MODEL,
                "input": [
                {
                    "role": "system",
                    "content": include_str!("../generate.prompt"),
                },
                {
                    "role": "user",
                    "content": user_prompt,
                },
                {
                    "role": "user",
                    "content": doc_content,
                }
                ],
            }))?;

        let res = res.body_mut().read_to_string()?;
        let res: Value = serde_json::from_str(&res)?;

        let gen_content = res["output"][0]["content"][0]["text"].to_string();
        let gen_content = gen_content
            .trim_start_matches('"')
            .trim_end_matches('"')
            .replace("\\n", "\n")
            .replace("\\\"", "\"");

        print!(
            "{}",
            log::info!(
                "obtenidos {} bytes de contenido generado",
                gen_content.len()
            )
        );

        Ok(gen_content)
    }
}
