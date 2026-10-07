// Copyright 2018 Parity Technologies (UK) Ltd.
//
// Permission is hereby granted, free of charge, to any person obtaining a
// copy of this software and associated documentation files (the "Software"),
// to deal in the Software without restriction, including without limitation
// the rights to use, copy, modify, merge, publish, distribute, sublicense,
// and/or sell copies of the Software, and to permit persons to whom the
// Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
// DEALINGS IN THE SOFTWARE.

//! Implementation of the [Yamux](https://github.com/hashicorp/yamux/blob/master/spec.md)  multiplexing protocol for libp2p.

#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]

//! Narrow public-upgrade glue adapted from libp2p-yamux0.48.0; maintained yamux owns all stream/credit logic.
use crate::records::Lane;
use futures::{AsyncRead, AsyncWrite, future, ready};
use libp2p::core::{
    muxing::{StreamMuxer, StreamMuxerEvent},
    upgrade::{InboundConnectionUpgrade, OutboundConnectionUpgrade, UpgradeInfo},
};
use std::{
    collections::VecDeque,
    io, iter,
    pin::Pin,
    task::{Context, Poll, Waker},
};
#[derive(Clone, Debug)]
pub struct LaneMuxConfig {
    inner: yamux::Config,
    streams: usize,
    window: usize,
    buffer: usize,
}
impl LaneMuxConfig {
    pub fn new(lane: Lane) -> Self {
        let streams = match lane {
            Lane::Control => 4,
            Lane::Bulk => 1,
        };
        let window = streams * 262144;
        let mut inner = yamux::Config::default();
        inner.set_max_num_streams(streams);
        inner.set_max_connection_receive_window(Some(window));
        Self {
            inner,
            streams,
            window,
            buffer: streams,
        }
    }
    pub fn stream_limit(&self) -> usize {
        self.streams
    }
    pub fn receive_window(&self) -> usize {
        self.window
    }
    pub fn buffer_limit(&self) -> usize {
        self.buffer
    }
    pub fn connection<C>(self, io: C, mode: yamux::Mode) -> LaneMux<C>
    where
        C: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        LaneMux {
            connection: yamux::Connection::new(io, self.inner, mode),
            buffer: VecDeque::new(),
            buffer_limit: self.buffer,
            waker: None,
        }
    }
}
impl UpgradeInfo for LaneMuxConfig {
    type Info = &'static str;
    type InfoIter = iter::Once<Self::Info>;
    fn protocol_info(&self) -> Self::InfoIter {
        iter::once("/yamux/1.0.0")
    }
}
impl<C> InboundConnectionUpgrade<C> for LaneMuxConfig
where
    C: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = LaneMux<C>;
    type Error = io::Error;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_inbound(self, io: C, _: Self::Info) -> Self::Future {
        future::ready(Ok(self.connection(io, yamux::Mode::Server)))
    }
}
impl<C> OutboundConnectionUpgrade<C> for LaneMuxConfig
where
    C: AsyncRead + AsyncWrite + Send + Unpin + 'static,
{
    type Output = LaneMux<C>;
    type Error = io::Error;
    type Future = future::Ready<Result<Self::Output, Self::Error>>;
    fn upgrade_outbound(self, io: C, _: Self::Info) -> Self::Future {
        future::ready(Ok(self.connection(io, yamux::Mode::Client)))
    }
}
pub struct LaneMux<C> {
    connection: yamux::Connection<C>,
    buffer: VecDeque<yamux::Stream>,
    buffer_limit: usize,
    waker: Option<Waker>,
}
impl<C> LaneMux<C>
where
    C: AsyncRead + AsyncWrite + Unpin + 'static,
{
    fn next(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<yamux::Stream, yamux::ConnectionError>> {
        Poll::Ready(
            ready!(self.connection.poll_next_inbound(cx)).ok_or(yamux::ConnectionError::Closed)?,
        )
    }
}
impl<C> StreamMuxer for LaneMux<C>
where
    C: AsyncRead + AsyncWrite + Unpin + 'static,
{
    type Substream = yamux::Stream;
    type Error = yamux::ConnectionError;
    fn poll_inbound(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        if let Some(s) = self.buffer.pop_front() {
            return Poll::Ready(Ok(s));
        }
        if let Poll::Ready(r) = self.next(cx) {
            return Poll::Ready(r);
        }
        self.waker = Some(cx.waker().clone());
        Poll::Pending
    }
    fn poll_outbound(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Substream, Self::Error>> {
        self.connection.poll_new_outbound(cx)
    }
    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.buffer.clear();
        self.connection.poll_close(cx)
    }
    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<StreamMuxerEvent, Self::Error>> {
        let inbound = ready!(self.next(cx))?;
        if self.buffer.len() < self.buffer_limit {
            self.buffer.push_back(inbound);
            if let Some(w) = self.waker.take() {
                w.wake();
            }
        } else {
            drop(inbound);
        }
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
