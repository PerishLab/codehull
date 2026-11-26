use crate::Error;
use radicula_process::asynchronous::{Limits, Runner};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::AsyncWrite;

const TIME: Duration = Duration::from_secs(600);
const DIAGNOSTIC: usize = 64 << 10;

pub const CAPTURE: Limits = Limits {
    time: TIME,
    output: 32 << 20,
};

pub const TRANSFER: Limits = Limits {
    time: TIME,
    output: 16 << 30,
};

pub fn runner(limits: Limits) -> Result<Runner, Error> {
    let held = if cfg!(target_os = "linux") {
        Runner::group(limits)
    } else {
        Runner::new(limits)
    };
    held.map_err(|error| Error::Io(format!("cannot configure git execution: {error}")))
}

#[derive(Default)]
pub struct Brief(Vec<u8>);

impl Brief {
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

impl AsyncWrite for Brief {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
        held: &[u8],
    ) -> Poll<io::Result<usize>> {
        let count = held.len().min(DIAGNOSTIC - self.0.len());
        self.0.extend_from_slice(&held[..count]);
        Poll::Ready(Ok(held.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}
