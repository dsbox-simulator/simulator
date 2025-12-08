use crate::process::Runner;
use crate::process::runner::{CommandReceiver, EventSender};

pub struct CallbackRunner<C> {
    callback: C,
}

impl<C> CallbackRunner<C> {
    pub fn new(callback: C) -> Self {
        Self { callback }
    }
}

pub struct CallbackOnceRunner<C> {
    callback: Option<C>,
}
impl<C, F> Runner for CallbackRunner<C>
where
    C: FnOnce(Vec<String>, EventSender, CommandReceiver) -> F + Clone,
    F: Future<Output = i32> + Send + 'static,
{
    fn run(
        &mut self,
        args: Vec<String>,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + Send + 'static {
        self.callback.clone()(args, sender, receiver)
    }
}

impl<C> CallbackOnceRunner<C> {
    pub fn new(callback: C) -> Self {
        Self {
            callback: Some(callback),
        }
    }
}

impl<C, F> Runner for CallbackOnceRunner<C>
where
    C: FnOnce(Vec<String>, EventSender, CommandReceiver) -> F,
    F: Future<Output = i32> + Send + 'static,
{
    fn run(
        &mut self,
        args: Vec<String>,
        sender: EventSender,
        receiver: CommandReceiver,
    ) -> impl Future<Output = i32> + Send + 'static {
        let fut = self
            .callback
            .take()
            .map(|callback| callback(args, sender, receiver));
        async move { if let Some(fut) = fut { fut.await } else { 0 } }
    }
}
