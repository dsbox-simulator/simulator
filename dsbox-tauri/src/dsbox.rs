use async_channel::{Receiver, Sender};
use dsbox_core::core::event::Event;
use dsbox_core::core::remote_control::RemoteCommand;
use dsbox_core::core::Core;
use dsbox_core::Command;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;

pub struct DsboxState {
    remote: Sender<RemoteCommand>,
    subscriber: Receiver<Event>,
    #[allow(unused)]
    core_handle: JoinHandle<()>,
    commands: Commands,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Commands {
    pub test_command: Command,
    pub server_command: Command,
}

impl DsboxState {
    pub fn new(args: crate::cli::Cli) -> Self {
        let test_command = Core::split_command(args.test_command.unwrap_or_default());
        let server_command = Core::make_command(args.server_command.unwrap_or_default());
        let core = Core::new(
            test_command.clone(),
            server_command.clone(),
            true,
            args.lua_unsafe,
        );
        Self {
            remote: core.remote_control(),
            subscriber: core.subscribe_events(),
            core_handle: tauri::async_runtime::spawn(async move { core.run().await }),
            commands: Commands {
                test_command,
                server_command,
            },
        }
    }
}

#[tauri::command]
pub async fn subscribe_events(
    state: tauri::State<'_, DsboxState>,
    on_event: Channel<Event>,
) -> tauri::Result<()> {
    while let Ok(event) = state.subscriber.recv().await {
        on_event.send(event)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn restart(
    state: tauri::State<'_, DsboxState>,
    test_command: Option<Command>,
    server_command: Option<Command>,
) -> tauri::Result<()> {
    state
        .remote
        .send(RemoteCommand::Restart {
            test_command,
            server_command,
        })
        .await
        .ok();
    Ok(())
}

#[tauri::command]
pub async fn break_(state: tauri::State<'_, DsboxState>) -> tauri::Result<()> {
    state.remote.send(RemoteCommand::Break).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn step(state: tauri::State<'_, DsboxState>) -> tauri::Result<()> {
    state.remote.send(RemoteCommand::Step).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: tauri::State<'_, DsboxState>) -> tauri::Result<()> {
    state.remote.send(RemoteCommand::Resume).await.ok();
    Ok(())
}

#[tauri::command]
pub fn current_commands(state: tauri::State<'_, DsboxState>) -> Commands {
    state.commands.clone()
}

#[tauri::command]
pub async fn deliver(
    state: tauri::State<'_, DsboxState>,
    sent_timestamp: usize,
) -> tauri::Result<()> {
    state
        .remote
        .send(RemoteCommand::Deliver(sent_timestamp))
        .await
        .ok();
    Ok(())
}

#[tauri::command]
pub async fn drop(state: tauri::State<'_, DsboxState>, sent_timestamp: usize) -> tauri::Result<()> {
    state
        .remote
        .send(RemoteCommand::Drop(sent_timestamp))
        .await
        .ok();
    Ok(())
}
