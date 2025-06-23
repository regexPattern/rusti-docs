use chrono::Local;
use redis_cmd::{
    Command,
    storage::{HSet, Set, StorageCommand},
};
use redis_resp::BulkString;

use crate::{DocsSyncer, error::Error};

impl DocsSyncer {
    /// Persiste el contenido de un documento en la base de datos Redis.
    /// Actualiza la timestamp de última edición.
    pub fn persist_document(
        &self,
        doc_id: String,
        doc_kind: String,
        doc_content: String,
    ) -> Result<(), Error> {
        let cmd = match doc_kind.as_str() {
            "TEXT" => self.persist_text_doc_cmd(&doc_id, &doc_content),
            "SPREADSHEET" => self.persist_spreadsheet_doc_cmd(&doc_id, &doc_content),
            _ => unreachable!(),
        };

        let cmd = Vec::from(Command::Storage(cmd));

        let mut slot_addr = self.db_addr;
        let (new_slot, _) = Self::cluster_command(slot_addr, cmd)?;
        slot_addr = new_slot;

        let cmd = Vec::from(Command::Storage(StorageCommand::HSet(HSet {
            key: "docs_ts".into(),
            field_value_pairs: vec![BulkString::from(&doc_id), Local::now().to_rfc3339().into()],
        })));

        let _ = Self::cluster_command(slot_addr, cmd)?;

        print!("{}", log::info!("persistido documento {}", doc_id));

        Ok(())
    }

    fn persist_text_doc_cmd(&self, doc_id: &str, doc_content: &str) -> StorageCommand {
        StorageCommand::Set(Set {
            key: BulkString::from(doc_id),
            value: BulkString::from(doc_content),
        })
    }

    fn persist_spreadsheet_doc_cmd(&self, doc_id: &str, doc_content: &str) -> StorageCommand {
        let mut field_value_pairs = Vec::new();

        for (i, value) in doc_content.split(',').enumerate() {
            let row = i / 10;
            let col = i % 10;

            field_value_pairs.push(BulkString::from(format!("{row},{col}")));
            field_value_pairs.push(BulkString::from(value));
        }

        StorageCommand::HSet(HSet {
            key: BulkString::from(doc_id),
            field_value_pairs,
        })
    }
}
