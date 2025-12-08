use dsbox_core::{CommandReceiver, EventSender, ProcessCommand, ProcessEvent, Runner};
use libproto::Message;
use libproto::init::Init;
use libproto::system::control::Control;
use libproto::system::event::{Event, PublishEvent, SubscribeEvents};
use tokio::sync::mpsc::{Receiver, Sender};

pub struct RemoteControl {
    pub remote_sender: Option<Sender<Event>>,
    pub remote_receiver: Option<Receiver<Control>>,
}

impl RemoteControl {
    pub fn new() -> (Self, Sender<Control>, Receiver<Event>) {
        let (to_core_sender, to_core_receiver) = tokio::sync::mpsc::channel(1);
        let (from_core_sender, from_core_receiver) = tokio::sync::mpsc::channel(1);
        (
            Self {
                remote_sender: Some(from_core_sender),
                remote_receiver: Some(to_core_receiver),
            },
            to_core_sender,
            from_core_receiver,
        )
    }

    async fn run_once(
        sender: EventSender,
        mut receiver: CommandReceiver,
        remote_sender: Sender<Event>,
        mut remote_receiver: Receiver<Control>,
    ) -> i32 {
        let mut from_core_open = true;
        let mut to_core_open = true;
        let mut own_name = None;
        let mut core_name = None;

        while from_core_open || to_core_open {
            tokio::select! {
                from_core = receiver.recv(), if from_core_open => {
                    let Some(from_core) = from_core else {from_core_open = false; continue;};
                    match from_core {
                        ProcessCommand::Deliver(message) => {
                            if let Ok(init) = message.payload::<Init>() {
                                // subscribe to events immediately after receiving the init message
                                sender
                                    .send(ProcessEvent::Message(Message::new(
                                        &init.name,
                                        &init.core_name,
                                        None,
                                        SubscribeEvents {},
                                    )))
                                    .await
                                    .ok();
                                own_name = Some(init.name);
                                core_name = Some(init.core_name);

                            } else if let Ok(publish) = message.payload::<PublishEvent>() {
                                remote_sender.send(publish.event).await.ok();
                            }
                        }
                        ProcessCommand::Shutdown => {
                            receiver.close();
                            remote_receiver.close();
                        }
                        ProcessCommand::Abort => break,

                    }
                }
                to_core = remote_receiver.recv(), if to_core_open => {
                    let Some(to_core) = to_core else { to_core_open = false; continue; };
                    let (Some(own_name), Some(core_name)) = (&own_name, &core_name) else {continue;};
                    sender.send(ProcessEvent::Message(Message::new(own_name, core_name, None, to_core))).await.ok();
                }
            }
        }
        log::trace!("remote control finished");
        0i32
    }
}

impl Runner for RemoteControl {
    fn run(
        &mut self,
        _: Vec<String>,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + Send + 'static {
        let remote_sender = self.remote_sender.take();
        let remote_receiver = self.remote_receiver.take();
        async move {
            let (Some(remote_sender), Some(remote_receiver)) = (remote_sender, remote_receiver)
            else {
                return 0;
            };
            Self::run_once(sender, receiver, remote_sender, remote_receiver).await
        }
    }
}
