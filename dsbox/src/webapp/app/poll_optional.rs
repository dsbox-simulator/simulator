use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::task::{JoinError, JoinHandle};

pub fn poll_optional_join_handle<T>(
    handle: &mut Option<JoinHandle<T>>,
) -> impl FnMut(&mut Context<'_>) -> Poll<Result<T, JoinError>> + use<'_, T> {
    move |ctx| match handle.take() {
        None => Poll::Pending,
        Some(mut join_handle) => match Pin::new(&mut join_handle).poll(ctx) {
            Poll::Pending => {
                *handle = Some(join_handle);
                Poll::Pending
            }
            ready => ready,
        },
    }
}