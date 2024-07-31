/*
 * Copyright 2024 ByteDance and/or its affiliates.
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

use std::io;

use tokio::io::AsyncWrite;

use g3_io_ext::LimitedWriteExt;

const BYE_BLOCKED: &str = "* BYE Blocked; connection not allowed\r\n";
const BYE_AUTO_LOGOUT: &str = "* BYE Autologout; idle for too long\r\n";
const BYE_INTERNAL_ERROR: &str = "* BYE [UNAVAILABLE] shutdown due to internal error\r\n";
const BYE_UPSTREAM_TIMEOUT: &str = "* BYE [UNAVAILABLE] timeout to recv upstream greeting\r\n";
const BYE_UPSTREAM_PROTOCOL_ERROR: &str = "* BYE [SERVERBUG] invalid upstream protocol\r\n";
const BYE_UPSTREAM_IO_ERROR: &str = "* BYE [UNAVAILABLE] connect to upstream failed\r\n";

pub struct ByeResponse {}

macro_rules! impl_method {
    ($method:ident, $message:ident) => {
        pub async fn $method<W>(writer: &mut W) -> io::Result<()>
        where
            W: AsyncWrite + Unpin,
        {
            writer.write_all_flush($message.as_bytes()).await
        }
    };
}

impl ByeResponse {
    impl_method!(reply_blocked, BYE_BLOCKED);
    impl_method!(reply_idle_logout, BYE_AUTO_LOGOUT);
    impl_method!(reply_internal_error, BYE_INTERNAL_ERROR);
    impl_method!(reply_upstream_timeout, BYE_UPSTREAM_TIMEOUT);
    impl_method!(reply_upstream_protocol_error, BYE_UPSTREAM_PROTOCOL_ERROR);
    impl_method!(reply_upstream_io_error, BYE_UPSTREAM_IO_ERROR);
}
