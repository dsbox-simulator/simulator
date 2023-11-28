use std::error::Error;

use libproto::echo::{Echo, EchoOk};
use libproto::init::Init;
use libproto::Message;

fn main() -> Result<(), Box<dyn Error>> {
    let message = Message::recv()
        .expect("expected init message")?;
    message.payload::<Init>()?;
    for msg in Message::recv_iter().map(Result::unwrap) {
        if let Ok(echo) = msg.payload::<Echo>() {
            msg.reply(None, EchoOk { echo: echo.echo }).send();
        };
    }
    Ok(())
}
