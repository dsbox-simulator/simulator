use libproto::Message;

pub struct ProcessEvent {
    pub source_id: usize,
    pub kind: ProcessEventKind,
}

pub enum ProcessEventKind {
    Message(Message),
    Log(String),
    Exited(i32),
    SerializeError(String, serde_json::Error),
}

impl ProcessEvent {
    pub fn new(source_id: usize, kind: ProcessEventKind) -> Self {
        Self {
            source_id,
            kind,
        }
    }
}