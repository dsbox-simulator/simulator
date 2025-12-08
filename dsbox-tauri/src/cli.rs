use app_lib::args;
use app_lib::dsbox::remote_control::RemoteControl;
use app_lib::dsbox::util::{guess_runner, register_runners};
use dsbox_core::{Capability, Core, RunnerCommand};
use enumflags2::BitFlags;
use libproto::system::control::Control;
use libproto::system::event::{Event, EventData};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::{Receiver, Sender};

pub fn run_cli(args: args::CliArgs) -> ExitCode {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_dsbox(args))
}

async fn run_dsbox(args: args::CliArgs) -> ExitCode {
    let (remote, sender, receiver) = RemoteControl::new();
    let builder = register_runners(Core::builder(), args.common.lua_unsafe, remote)
        .interactive(false)
        .register_command(
            "remote",
            RunnerCommand::new("remote_control", vec!["remote_control".to_owned()]),
            BitFlags::default() | Capability::SubscribeEvents | Capability::ControlCore,
        )
        .register_command(
            "test",
            guess_runner(&args.test_command),
            BitFlags::default()
                | Capability::LaunchNodes
                | Capability::LaunchAlias
                | Capability::Monitor
                | Capability::Reset,
        )
        .register_command(
            "server",
            guess_runner(&args.server_command.join(" ")),
            BitFlags::default(),
        )
        .launch_weak("remote", "remote", false)
        .launch("test", "test", true);

    let exit_code = Arc::new(AtomicU8::new(0));

    let recorder_task = {
        let exit_code = Arc::clone(&exit_code);
        tokio::task::spawn(async move {
            recorder(sender, receiver, exit_code, args.save_protocol).await;
        })
    };

    let core = builder.build();

    core.run().await;
    recorder_task.await.ok();

    ExitCode::from(exit_code.load(Ordering::Relaxed))
}

async fn recorder(
    _: Sender<Control>,
    mut receiver: Receiver<Event>,
    exit_code: Arc<AtomicU8>,
    protocol_file: Option<String>,
) {
    let mut protocol_writer = if let Some(protocol_file) = &protocol_file {
        match File::open(protocol_file).await {
            Ok(f) => Some(f),
            Err(e) => {
                log::warn!("failed to open protocol file `{protocol_file}`: {e}");
                None
            }
        }
    } else {
        None
    };

    while let Some(event) = receiver.recv().await {
        if let EventData::NodeDisconnected {
            name,
            exit_code: code,
        } = &event.data
            && name == "test"
        {
            exit_code.store(code.unwrap_or(-1) as u8, Ordering::Relaxed);
        }
        if let Some(writer) = &mut protocol_writer {
            let mut event_str = serde_json::to_string(&event).unwrap();
            event_str.push('\n');
            if let Err(e) = writer.write_all(event_str.as_bytes()).await {
                log::warn!(
                    "failed to write event to protocol file `{}`: {e}",
                    protocol_file.as_ref().unwrap()
                );
                protocol_writer.take();
            }
        }
    }
    if let Some(protocol_writer) = &mut protocol_writer {
        protocol_writer.flush().await.ok();
    }
}
