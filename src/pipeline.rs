use async_stream::stream;
use chrono::Local;
use futures_core::Stream;
use lockfree_object_pool::{LinearObjectPool, LinearOwnedReusable};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    maybe_multicast_socket::MaybeMulticastReceiver, payload::Payload, utils::as_mut_u8_slice,
};

pub fn receive_pkt(
    socket: MaybeMulticastReceiver,
) -> impl Stream<Item = LinearOwnedReusable<Payload>> {
    let mut last_print_time = Instant::now();
    let print_interval = Duration::from_secs(2);

    let mut next_cnt = None;
    let mut ndropped = 0;
    let mut nreceived = 0;
    let pool: Arc<LinearObjectPool<Payload>> = Arc::new(LinearObjectPool::new(
        move || {
            //eprint!("o");
            Payload::default()
        },
        |v| {
            v.pkt_cnt = 0;
            v.data.fill(0);
        },
    ));
    //socket.set_nonblocking(true).unwrap();

    stream! {
        loop {
            let mut payload = pool.pull_owned();
            let buf = as_mut_u8_slice(&mut payload as &mut Payload);

            match socket.recv_from(buf).await {
                Ok((s, _a)) => {
                    if s != std::mem::size_of::<Payload>() {
                        continue;
                    }
                }
                _ => continue,
            }

            let now = Instant::now();

            if now.duration_since(last_print_time) >= print_interval {
                let local_time = Local::now().format("%Y-%m-%d %H:%M:%S");
                println!(
                    "{local_time} {ndropped} pkts dropped, ratio<{:e}",
                    (1 + ndropped) as f64 / nreceived as f64
                );
                last_print_time = now;
            }

            if next_cnt.is_none() {
                next_cnt = Some(payload.pkt_cnt);
                ndropped = 0;
            }

            if payload.pkt_cnt == 0 {
                ndropped = 0;
                nreceived = 0;
                let local_time = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
                println!();
                println!("==================================");
                println!("start time:{local_time}");
                println!("==================================");
            }

            while let Some(ref mut c) = next_cnt {
                //let current_cnt = c + 1;
                if *c >= payload.pkt_cnt {
                    //actually = is sufficient.
                    *c = payload.pkt_cnt + 1;

                    nreceived += 1;
                    yield payload;
                    break;
                }

                ndropped += 1;

                let mut payload1 = pool.pull_owned();
                payload1.copy_header(&payload);
                payload1.pkt_cnt = *c;

                nreceived += 1;
                yield payload1;
                *c += 1;

                if ndropped % 1000 == 0 {
                    tokio::task::yield_now().await;
                }
            }
        }
    }
}
