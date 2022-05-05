use std::sync::Arc;

use criterion::*;
use madsim::{net::Endpoint, Request};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Request)]
#[rtype("()")]
struct Req;

fn empty_rpc(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let addr = rt.block_on(async move {
        let net = Arc::new(Endpoint::bind("127.0.0.1:0").await.unwrap());
        net.add_rpc_handler(|_: Req| async move {});
        net.local_addr().unwrap()
    });

    c.bench_function("empty RPC", |b| {
        let net = rt.block_on(Endpoint::bind("127.0.0.1:0")).unwrap();
        b.iter(|| rt.block_on(net.call(addr, Req)).unwrap());
    });
}

fn rpc_data(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let addr = rt.block_on(async move {
        let net = Arc::new(Endpoint::bind("127.0.0.1:0").await.unwrap());
        net.add_rpc_handler_with_data(|_: Req, data| async move {
            black_box(data);
            ((), vec![])
        });
        net.local_addr().unwrap()
    });

    let mut group = c.benchmark_group("RPC with data");
    group.plot_config(PlotConfiguration::default().summary_scale(AxisScale::Logarithmic));
    for size in [16, 256, 4096, 65536, 1048576] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let data = vec![0u8; size];
            let net = rt.block_on(Endpoint::bind("127.0.0.1:0")).unwrap();
            b.iter(|| rt.block_on(net.call_with_data(addr, Req, &data)).unwrap());
        });
    }
    group.finish();
}

criterion_group!(benches, empty_rpc, rpc_data);
criterion_main!(benches);
