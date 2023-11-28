use libproto::Message;

pub enum ProcessCommand {
    Deliver(Message),
}