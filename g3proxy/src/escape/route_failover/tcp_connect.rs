/*
 * Copyright 2023 ByteDance and/or its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use std::pin::pin;

use anyhow::anyhow;

use g3_daemon::stat::remote::ArcTcpConnectionTaskRemoteStats;
use g3_types::net::UpstreamAddr;

use super::RouteFailoverEscaper;
use crate::escape::ArcEscaper;
use crate::module::tcp_connect::{TcpConnectError, TcpConnectResult, TcpConnectTaskNotes};
use crate::serve::ServerTaskNotes;

pub struct TcpConnectFailoverContext {
    tcp_notes: TcpConnectTaskNotes,
    connect_result: TcpConnectResult,
}

impl TcpConnectFailoverContext {
    fn new(upstream: &UpstreamAddr) -> Self {
        let tcp_notes = TcpConnectTaskNotes::new(upstream.clone());
        TcpConnectFailoverContext {
            tcp_notes,
            connect_result: Err(TcpConnectError::EscaperNotUsable(anyhow!(
                "tcp setup connection not called yet"
            ))),
        }
    }

    async fn run(
        mut self,
        escaper: &ArcEscaper,
        task_notes: &ServerTaskNotes,
        task_stats: ArcTcpConnectionTaskRemoteStats,
    ) -> Result<Self, Self> {
        match escaper
            .tcp_setup_connection(&mut self.tcp_notes, task_notes, task_stats)
            .await
        {
            Ok(c) => {
                self.connect_result = Ok(c);
                Ok(self)
            }
            Err(e) => {
                self.connect_result = Err(e);
                Err(self)
            }
        }
    }
}

impl RouteFailoverEscaper {
    pub(super) async fn tcp_setup_connection_with_failover<'a>(
        &'a self,
        tcp_notes: &'a mut TcpConnectTaskNotes,
        task_notes: &'a ServerTaskNotes,
        task_stats: ArcTcpConnectionTaskRemoteStats,
    ) -> TcpConnectResult {
        let primary_context = TcpConnectFailoverContext::new(&tcp_notes.upstream);
        let mut primary_task =
            pin!(primary_context.run(&self.primary_node, task_notes, task_stats.clone()));

        match tokio::time::timeout(self.config.fallback_delay, &mut primary_task).await {
            Ok(Ok(ctx)) => {
                self.stats.add_request_passed();
                tcp_notes.fill_generated(&ctx.tcp_notes);
                return ctx.connect_result;
            }
            Ok(Err(_)) => {
                return match self
                    .standby_node
                    .tcp_setup_connection(tcp_notes, task_notes, task_stats)
                    .await
                {
                    Ok(c) => {
                        self.stats.add_request_passed();
                        Ok(c)
                    }
                    Err(e) => {
                        self.stats.add_request_failed();
                        Err(e)
                    }
                }
            }
            Err(_) => {}
        }

        let standby_context = TcpConnectFailoverContext::new(&tcp_notes.upstream);
        let standby_task = pin!(standby_context.run(&self.standby_node, task_notes, task_stats));

        match futures_util::future::select_ok([primary_task, standby_task]).await {
            Ok((ctx, _left)) => {
                self.stats.add_request_passed();
                tcp_notes.fill_generated(&ctx.tcp_notes);
                ctx.connect_result
            }
            Err(ctx) => {
                self.stats.add_request_failed();
                tcp_notes.fill_generated(&ctx.tcp_notes);
                ctx.connect_result
            }
        }
    }
}
