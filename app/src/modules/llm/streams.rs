use async_openai::types::CompletionUsage;
use async_std::channel::{Sender, unbounded};
use async_std::sync::Mutex;
use futures::{Stream, StreamExt, stream};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Progress of a single tool invocation made by the model, streamed to
/// subscribers as it starts and again when it resolves (matched by `call_id`).
#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct ToolCallUpdate {
    pub call_id: String,
    pub tool: String,
    pub label: String,
    /// "running", "ok", or "error"
    pub status: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Chunk(String),
    /// Coarse phase milestone, e.g. "Writing summary".
    ToolActivity(String),
    ToolCall(ToolCallUpdate),
    /// Subscribers must discard all previously received content (bad output
    /// was abandoned and generation is being retried).
    Reset,
    Done(Result<(), String>),
}

#[derive(Default)]
struct StreamInner {
    history: Vec<StreamEvent>,
    senders: Vec<Sender<StreamEvent>>,
    done: Option<Result<String, String>>,
    usage: Option<CompletionUsage>,
}

impl StreamInner {
    fn done_event(&self) -> Option<StreamEvent> {
        self.done
            .as_ref()
            .map(|result| StreamEvent::Done(result.as_ref().map(|_| ()).map_err(Clone::clone)))
    }

    fn broadcast(&mut self, event: StreamEvent) {
        self.history.push(event.clone());
        self.senders
            .retain(|sender| sender.try_send(event.clone()).is_ok());
    }
}

#[derive(Clone, Default)]
pub struct SharedStream {
    inner: Arc<Mutex<StreamInner>>,
}

impl SharedStream {
    pub async fn publish(&self, chunk: String) {
        self.inner.lock().await.broadcast(StreamEvent::Chunk(chunk));
    }

    pub async fn publish_tool_activity(&self, activity: String) {
        self.inner
            .lock()
            .await
            .broadcast(StreamEvent::ToolActivity(activity));
    }

    pub async fn publish_tool_call(&self, update: ToolCallUpdate) {
        self.inner
            .lock()
            .await
            .broadcast(StreamEvent::ToolCall(update));
    }

    pub async fn publish_reset(&self) {
        let mut inner = self.inner.lock().await;
        // Late subscribers should never replay the abandoned content.
        inner
            .history
            .retain(|event| !matches!(event, StreamEvent::Chunk(_)));
        inner.broadcast(StreamEvent::Reset);
    }

    pub async fn finish(&self, result: Result<String, String>, usage: Option<CompletionUsage>) {
        let mut inner = self.inner.lock().await;
        inner.done = Some(result);
        inner.usage = usage;
        let event = inner.done_event().expect("done was just set");
        for sender in inner.senders.drain(..) {
            let _ = sender.try_send(event.clone());
        }
    }

    /// Replays the full ordered event history, then live events until done.
    pub async fn subscribe(&self) -> impl Stream<Item = StreamEvent> + Send + 'static {
        let (sender, receiver) = unbounded();
        let replay = {
            let mut inner = self.inner.lock().await;
            let mut events = inner.history.clone();
            match inner.done_event() {
                Some(done) => events.push(done),
                None => inner.senders.push(sender),
            }
            events
        };

        stream::iter(replay).chain(receiver)
    }

    pub async fn await_completion(&self) -> Result<String, String> {
        let mut events = self.subscribe().await;
        while let Some(event) = events.next().await {
            if let StreamEvent::Done(result) = event {
                result?;
                let inner = self.inner.lock().await;
                return match &inner.done {
                    Some(Ok(content)) => Ok(content.clone()),
                    _ => Err("stream completed without content".to_string()),
                };
            }
        }

        Err("stream closed before completion".to_string())
    }

    pub async fn usage(&self) -> Option<CompletionUsage> {
        self.inner.lock().await.usage.clone()
    }

    pub async fn is_done(&self) -> bool {
        self.inner.lock().await.done.is_some()
    }

    pub fn same_stream(&self, other: &SharedStream) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

#[derive(Default)]
pub struct StreamRegistry {
    streams: Mutex<std::collections::HashMap<String, SharedStream>>,
}

impl StreamRegistry {
    pub async fn get(&self, key: &str) -> Option<SharedStream> {
        self.streams.lock().await.get(key).cloned()
    }

    /// Returns the registered stream only while its run is still in flight.
    /// A finished stream lingering in its post-completion grace period must
    /// never block a new generation from starting.
    pub async fn get_or_create(
        &self,
        key: &str,
        create: impl FnOnce() -> SharedStream,
    ) -> (SharedStream, bool) {
        let mut streams = self.streams.lock().await;
        if let Some(existing) = streams.get(key)
            && !existing.is_done().await
        {
            return (existing.clone(), false);
        }
        let stream = create();
        streams.insert(key.to_string(), stream.clone());
        (stream, true)
    }

    /// Removes the key only if it still maps to `stream`, so a delayed cleanup
    /// can never unregister a newer run that reused the key.
    pub async fn remove_if(&self, key: &str, stream: &SharedStream) {
        let mut streams = self.streams.lock().await;
        if let Some(existing) = streams.get(key)
            && existing.same_stream(stream)
        {
            streams.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task::block_on;

    #[test]
    fn coalesces_only_onto_in_flight_streams() {
        block_on(async {
            let registry = StreamRegistry::default();

            let (first, created) = registry.get_or_create("k", SharedStream::default).await;
            assert!(created);

            let (again, created) = registry.get_or_create("k", SharedStream::default).await;
            assert!(!created);
            assert!(again.same_stream(&first));

            first.finish(Ok("done".to_string()), None).await;

            // A finished stream must not block a new run under the same key.
            let (second, created) = registry.get_or_create("k", SharedStream::default).await;
            assert!(created);
            assert!(!second.same_stream(&first));
        });
    }

    #[test]
    fn delayed_cleanup_cannot_remove_a_newer_stream() {
        block_on(async {
            let registry = StreamRegistry::default();

            let (first, _) = registry.get_or_create("k", SharedStream::default).await;
            first.finish(Ok(String::new()), None).await;
            let (second, _) = registry.get_or_create("k", SharedStream::default).await;

            // The first run's grace-period cleanup fires after the second run
            // took over the key: it must be a no-op.
            registry.remove_if("k", &first).await;
            let current = registry.get("k").await.expect("second stream still registered");
            assert!(current.same_stream(&second));

            registry.remove_if("k", &second).await;
            assert!(registry.get("k").await.is_none());
        });
    }
}
