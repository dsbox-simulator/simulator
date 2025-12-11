use async_channel::Receiver;
use dsbox_core::core::Core;
use dsbox_core::core::event::Event;
use std::process::ExitCode;
use tokio::task::JoinHandle;

pub fn run_cli(args: crate::args::CliArgs, allow_lua_unsafe: bool) -> ExitCode {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run_dsbox(args, allow_lua_unsafe))
}

async fn run_dsbox(args: crate::args::CliArgs, allow_lua_unsafe: bool) -> ExitCode {
    let core = Core::builder(
        Core::split_command(args.test_command),
        Core::make_command(args.server_command),
    )
    .interactive(false)
    .allow_lua_unsafe(allow_lua_unsafe)
    .build();

    let recorder = if let Some(filename) = args.save_protocol {
        Some(spawn_protocol_recorder(core.subscribe_events(), filename).await)
    } else {
        None
    };

    let exit_code = core.run().await;

    if let Some(recorder) = recorder {
        recorder.await.ok();
    }
    ExitCode::from(exit_code as u8)
}

async fn spawn_protocol_recorder(
    subscriber: Receiver<Event>,
    output_file: String,
) -> JoinHandle<()> {
    use tokio::io::AsyncWriteExt;
    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_file)
        .await
        .expect("failed to open protocol output file");

    tokio::task::spawn(async move {
        loop {
            let event = subscriber.recv().await;
            let Ok(event) = event else {
                break;
            };
            file.write_all(
                serde_json::to_string(&event)
                    .expect("failed to serialize event for protocol file")
                    .as_bytes(),
            )
            .await
            .expect("failed to write to protocol file");
            file.write_all(b"\n")
                .await
                .expect("failed to write to protocol file");
        }
    })
}
