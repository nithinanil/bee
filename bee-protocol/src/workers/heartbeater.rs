// Copyright 2020-2021 IOTA Stiftung
// SPDX-License-Identifier: Apache-2.0

use crate::{
    types::metrics::NodeMetrics,
    workers::{helper, peer::PeerManager, storage::StorageBackend, MetricsWorker, PeerManagerResWorker},
};

use bee_runtime::{node::Node, shutdown_stream::ShutdownStream, worker::Worker};
use bee_tangle::{MsTangle, TangleWorker};

use async_trait::async_trait;
use futures::stream::StreamExt;
use log::info;
use tokio::time::interval;
use tokio_stream::wrappers::IntervalStream;

use std::{any::TypeId, convert::Infallible, time::Duration};

const _HEARTBEAT_SEND_INTERVAL: u64 = 30; // In seconds.
const _HEARTBEAT_RECEIVE_INTERVAL: u64 = 100; // In seconds.
const CHECK_HEARTBEATS_INTERVAL: u64 = 5; // In seconds.

#[derive(Default)]
pub(crate) struct HeartbeaterWorker {}

#[async_trait]
impl<N: Node> Worker<N> for HeartbeaterWorker
where
    N::Backend: StorageBackend,
{
    type Config = ();
    type Error = Infallible;

    fn dependencies() -> &'static [TypeId] {
        vec![
            TypeId::of::<TangleWorker>(),
            TypeId::of::<PeerManagerResWorker>(),
            TypeId::of::<MetricsWorker>(),
        ]
        .leak()
    }

    async fn start(node: &mut N, _config: Self::Config) -> Result<Self, Self::Error> {
        let tangle = node.resource::<MsTangle<N::Backend>>();
        let peer_manager = node.resource::<PeerManager>();
        let metrics = node.resource::<NodeMetrics>();

        node.spawn::<Self, _, _>(|shutdown| async move {
            info!("Running.");

            let mut ticker = ShutdownStream::new(
                shutdown,
                IntervalStream::new(interval(Duration::from_secs(CHECK_HEARTBEATS_INTERVAL))),
            );

            while ticker.next().await.is_some() {
                // TODO real impl
                helper::broadcast_heartbeat(&peer_manager, &metrics, &tangle).await;
            }

            info!("Stopped.");
        });

        Ok(Self::default())
    }
}
