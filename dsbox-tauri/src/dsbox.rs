use async_channel::{Receiver, Sender};
use dsbox_core::command::RunnerCommand;
use dsbox_core::core::event::Event;
use dsbox_core::core::remote_control::RemoteCommand;
use dsbox_core::core::Core;
use serde::Serialize;
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tokio::sync::RwLock;

pub struct DsboxState {
    remote: Sender<RemoteCommand>,
    subscriber: Receiver<Event>,
    #[allow(unused)]
    core_handle: JoinHandle<()>,
    commands: Commands,
    lua_unsafe: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Commands {
    pub test_command: String,
    pub server_command: String,
}

impl DsboxState {
    pub fn new(commands: Commands, lua_unsafe: bool) -> Self {

        let core = Core::builder(
            commands.test_command.clone(),
            commands.server_command.clone(),
        )
        .interactive(true)
        .allow_lua_unsafe(lua_unsafe)
        .build();
        Self {
            remote: core.remote_control(),
            subscriber: core.subscribe_events(),
            core_handle: tauri::async_runtime::spawn(async move {
                let _ = core.run().await;
            }),
            commands,
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
        let core = state.read().await;
        eprintln!("subscribing to events on core @{:p}", &*core);
        core.subscriber.clone()
    };
    while let Ok(event) = receiver.recv().await {
        on_event.send(event)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn restart(
    state: tauri::State<'_, RwLock<DsboxState>>,
    test_command: Option<RunnerCommand>,
    server_command: Option<RunnerCommand>,
) -> tauri::Result<()> {
    let mut core = state.write().await;
    core.remote.send(RemoteCommand::Shutdown).await.ok();
    let commands = Commands {
        test_command: test_command.unwrap_or_else(|| core.commands.test_command.clone()),
        server_command: server_command.unwrap_or_else(|| core.commands.server_command.clone()),
    };

    *core = DsboxState::new(commands, core.lua_unsafe);
    Ok(())
}

#[tauri::command]
pub async fn break_(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.remote.send(RemoteCommand::Break).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn step(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.remote.send(RemoteCommand::Step).await.ok();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: tauri::State<'_, RwLock<DsboxState>>) -> tauri::Result<()> {
    let core = state.read().await;
    core.remote.send(RemoteCommand::Resume).await.ok();
    Ok(())
}

#[tauri::command]
pub fn current_commands(state: tauri::State<'_, RwLock<DsboxState>>) -> Commands {
    let core = state.blocking_read();
    core.commands.clone()
}

#[tauri::command]
pub async fn deliver(
    state: tauri::State<'_, RwLock<DsboxState>>,
    sent_timestamp: usize,
) -> tauri::Result<()> {
    let core = state.read().await;
    core.remote
        .send(RemoteCommand::Deliver(sent_timestamp))
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
    core.remote
        .send(RemoteCommand::Drop(sent_timestamp))
        .await
        .ok();
    Ok(())
}
