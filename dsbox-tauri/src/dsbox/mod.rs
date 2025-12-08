use crate::dsbox::util::{guess_runner, register_runners};
use dsbox_core::{Capability, Core, RunnerCommand};
use enumflags2::BitFlags;
use libproto::system::control::Control;
use libproto::system::event::Event;
use remote_control::RemoteControl;
use std::time::Duration;
use tauri::ipc::Channel;
use tokio::sync::RwLock;
use tokio::sync::mpsc::{Receiver, Sender};
use tauri::async_runtime::JoinHandle;

pub mod interpreters;
pub mod remote_control;
pub mod util;

pub struct DsboxState {
    sender: Sender<Control>,
    receiver: Option<Receiver<Event>>,
    handle: JoinHandle<()>,
    test_command: String,
    server_command: String,
    lua_unsafe: bool,
}

impl DsboxState {
    pub fn new(test_command: String, server_command: String, lua_unsafe: bool) -> Self {
        let (remote, sender, receiver) = RemoteControl::new();

        let mut builder = register_runners(Core::builder(), lua_unsafe, remote)
            .interactive(true)
            .register_command(
                "remote",
                RunnerCommand::new("remote_control", vec!["remote_control".to_owned()]),
                BitFlags::default() | Capability::SubscribeEvents | Capability::ControlCore,
            )
            .launch_weak("remote", "remote", false);

        if !test_command.is_empty() {
            builder = builder
                .register_command(
                    "test",
                    guess_runner(&test_command),
                    BitFlags::default()
                        | Capability::LaunchNodes
                        | Capability::LaunchAlias
                        | Capability::Monitor
                        | Capability::Reset,
                )
                .launch("test", "test", true);
        }

        if !server_command.is_empty() {
            builder = builder.register_command(
                "server",
                guess_runner(&server_command),
                BitFlags::default(),
            );
        }

        let core = builder.build();

        Self {
            sender,
            receiver: Some(receiver),
            handle: tauri::async_runtime::spawn(async move { core.run().await }),
            test_command,
            server_command,
            lua_unsafe,
        }
    }
}

#[tauri::command]
pub async fn subscribe_events(
    state: tauri::State<'_, RwLock<DsboxState>>,
    on_event: Channel<Event>,
) -> tauri::Result<()> {
    let receiver = {
        let mut core = state.write().await;
        core.receiver.take()
    };
    let Some(mut receiver) = receiver else {
        // TODO: communicate the error?
        return Ok(());
    };

    while let Some(event) = receiver.recv().await {
        on_event.send(event)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn restart(
    state: tauri::State<'_, RwLock<DsboxState>>,
    test_command: Option<String>,
    server_command: Option<String>,
) -> tauri::Result<()> {
    let mut core = state.write().await;
    core.sender.send(Control::Shutdown).await.ok();
    if tokio::time::timeout(Duration::from_secs(1), &mut core.handle)
        .await
        .is_err()
    {
        log::warn!("core did not finish gracefully");
    }

    let test_command = test_command.unwrap_or_else(|| core.test_command.clone());
    let server_command = server_command.unwrap_or_else(|| core.server_command.clone());
    *core = DsboxState::new(test_command, server_command, core.lua_unsafe);
    Ok(())
}

#[tauri::command]
pub async fn break_(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.sender.send(Control::Break).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn step(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.sender.send(Control::Step).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.sender.send(Control::Resume).await.ok();
    Ok(())
}

#[tauri::command]
pub fn current_commands(state: tauri::State<'_, RwLock<DsboxState>>) -> (String, String) {
    let core = state.blocking_read();
    (core.test_command.clone(), core.server_command.clone())
}

#[tauri::command]
pub async fn deliver(
    state: tauri::State<'_, RwLock<DsboxState>>,
    sent_timestamp: usize,
) -> tauri::Result<()> {
    let core = state.read().await;
    core.sender
        .send(Control::Deliver { sent_timestamp })
        .await
        .ok();
    Ok(())
}

#[tauri::command]
pub async fn drop(
    state: tauri::State<'_, RwLock<DsboxState>>,
    sent_timestamp: usize,
) -> tauri::Result<()> {
    let core = state.read().await;
    core.sender
        .send(Control::Drop { sent_timestamp })
        .await
        .ok();
    Ok(())
}
