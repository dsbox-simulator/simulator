use std::error::Error;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use libproto::Message;
use libproto::Payload;
use libproto::system::{Setup, SetupOk};

#[derive(Payload, Serialize, Deserialize)]
struct Empty {}

fn main() -> Result<(), Box<dyn Error>> {
    Message::new("client", "core", None, Setup { clients: vec!["c".to_owned()], servers: vec![] }).send();
    Message::recv().unwrap().unwrap().payload::<SetupOk>().unwrap();

    const NUM_TRIPS: u32 = 100;

    'outer: loop {
        let mut min = Duration::MAX;
        let mut max = Duration::ZERO;
        let mut total = Duration::ZERO;

        for _ in 0..NUM_TRIPS {
            let before = Instant::now();
            Message::new("c", "c", None, Empty {})
                .send();
            let msg_in = Message::recv()
                .expect("expected message to be delivered to self")?;
            let roundtrip_time = Instant::now().duration_since(before);
            if msg_in.payload::<Empty>().is_err() {
                eprintln!("round trip failed");
                break 'outer;
            }
            min = min.min(roundtrip_time);
            max = max.max(roundtrip_time);
            total += roundtrip_time;
        }

        let avg = total / NUM_TRIPS;
        eprintln!("round trips ok: avg {:.3}ms, min {:.3}ms, max {:.3}ms",
                  avg.as_secs_f64() * 1000.0,
                  min.as_secs_f64() * 1000.0,
                  max.as_secs_f64() * 1000.0);
    }
    Ok(())
}