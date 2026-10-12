//! Idle deadlines for streaming request-body reads.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use hyper::body::{Body, Frame, SizeHint};

pub(super) struct IdleTimeoutBody<B> {
    inner: B,
    timeout: Duration,
    timer: Option<Pin<Box<tokio::time::Sleep>>>,
    expired: Arc<AtomicBool>,
    finished: bool,
}

impl<B> IdleTimeoutBody<B> {
    pub(super) fn new(inner: B, timeout: Duration, expired: Arc<AtomicBool>) -> Self {
        Self {
            inner,
            timeout,
            timer: None,
            expired,
            finished: false,
        }
    }
}

impl<B> Body for IdleTimeoutBody<B>
where
    B: Body + Unpin,
    B::Error: Into<axum::BoxError>,
{
    type Data = B::Data;
    type Error = axum::BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        if self.finished {
            return Poll::Ready(None);
        }
        match Pin::new(&mut self.inner).poll_frame(cx) {
            Poll::Ready(frame) => {
                self.timer = None;
                self.finished = matches!(frame, None | Some(Err(_)));
                Poll::Ready(frame.map(|frame| frame.map_err(Into::into)))
            }
            Poll::Pending => {
                // Start only when the consumer actually waits for input. Pausing
                // a stream to do application work does not start an idle timer.
                let timeout = self.timeout;
                let timer = self
                    .timer
                    .get_or_insert_with(|| Box::pin(tokio::time::sleep(timeout)));
                if timer.as_mut().poll(cx).is_pending() {
                    return Poll::Pending;
                }
                self.finished = true;
                self.timer = None;
                self.expired.store(true, Ordering::Relaxed);
                Poll::Ready(Some(Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "request body read timed out",
                )
                .into())))
            }
        }
    }

    fn is_end_stream(&self) -> bool {
        self.finished || self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        if self.finished {
            SizeHint::with_exact(0)
        } else {
            self.inner.size_hint()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::body::Bytes;

    struct ChannelBody(tokio::sync::mpsc::UnboundedReceiver<Result<Frame<Bytes>, std::io::Error>>);

    impl Body for ChannelBody {
        type Data = Bytes;
        type Error = std::io::Error;

        fn poll_frame(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Bytes>, Self::Error>>> {
            self.0.poll_recv(cx)
        }
    }

    #[tokio::test]
    async fn stalled_body_returns_a_terminal_timeout() {
        let (_sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let expired = Arc::new(AtomicBool::new(false));
        let mut body = IdleTimeoutBody::new(
            ChannelBody(receiver),
            Duration::from_millis(20),
            Arc::clone(&expired),
        );
        let error = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap_err();
        assert_eq!(
            error.downcast_ref::<std::io::Error>().unwrap().kind(),
            std::io::ErrorKind::TimedOut
        );
        assert!(expired.load(Ordering::Relaxed));
        assert!(body.is_end_stream());
        assert!(
            std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn progressing_frames_reset_the_idle_deadline() {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let expired = Arc::new(AtomicBool::new(false));
        let mut body = IdleTimeoutBody::new(
            ChannelBody(receiver),
            Duration::from_millis(60),
            Arc::clone(&expired),
        );
        let producer = tokio::spawn(async move {
            for _ in 0..4 {
                tokio::time::sleep(Duration::from_millis(25)).await;
                sender
                    .send(Ok(Frame::data(Bytes::from_static(b"x"))))
                    .unwrap();
            }
        });
        let mut received = Vec::new();
        while let Some(frame) = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx)).await
        {
            received.extend_from_slice(&frame.unwrap().into_data().unwrap());
        }
        producer.await.unwrap();
        assert_eq!(received, b"xxxx");
        assert!(!expired.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn application_pauses_do_not_expire_ready_frames() {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let expired = Arc::new(AtomicBool::new(false));
        let mut body = IdleTimeoutBody::new(
            ChannelBody(receiver),
            Duration::from_millis(20),
            Arc::clone(&expired),
        );
        sender
            .send(Ok(Frame::data(Bytes::from_static(b"first"))))
            .unwrap();
        let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frame.into_data().unwrap(), b"first".as_slice());
        tokio::time::sleep(Duration::from_millis(40)).await;
        sender
            .send(Ok(Frame::data(Bytes::from_static(b"second"))))
            .unwrap();
        let frame = std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frame.into_data().unwrap(), b"second".as_slice());
        drop(sender);
        assert!(
            std::future::poll_fn(|cx| Pin::new(&mut body).poll_frame(cx))
                .await
                .is_none()
        );
        assert!(!expired.load(Ordering::Relaxed));
    }
}
