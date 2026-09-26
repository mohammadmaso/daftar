//! Retry with backoff for transient provider failures (§9 "rate-limit and retry with backoff").
//! Wraps any provider. A streamed chat is only retried if nothing was streamed yet, so the user
//! never sees an answer start twice. Longer outages are handled by the job queue's own backoff.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::{ChatRequest, ChatResponse, DynProvider, LlmProvider, OnDelta, ProviderResult};

pub struct Retrying {
    inner: DynProvider,
    attempts: u32,
    base: Duration,
    /// Longest wait honoured from a `Retry-After` header before giving the job back to the queue.
    max_wait: Duration,
}

impl Retrying {
    pub fn new(inner: DynProvider) -> Self {
        Self {
            inner,
            attempts: 3,
            base: Duration::from_millis(800),
            max_wait: Duration::from_secs(20),
        }
    }

    #[cfg(test)]
    fn fast(inner: DynProvider) -> Self {
        Self {
            base: Duration::from_millis(1),
            max_wait: Duration::from_millis(5),
            ..Self::new(inner)
        }
    }

    fn delay(&self, attempt: u32, retry_after_s: Option<u64>) -> Option<Duration> {
        let d = match retry_after_s {
            Some(s) => Duration::from_secs(s),
            None => self.base * 2u32.pow(attempt),
        };
        (d <= self.max_wait).then_some(d)
    }

    async fn run<T, F, Fut>(&self, mut call: F) -> ProviderResult<T>
    where
        F: FnMut() -> Fut,
        Fut: Future<Output = ProviderResult<T>>,
    {
        let mut attempt = 0;
        loop {
            match call().await {
                Err(e) if e.transient() && attempt + 1 < self.attempts => {
                    let Some(d) = self.delay(attempt, e.retry_after_s) else {
                        return Err(e);
                    };
                    tracing::info!("{}: {} — retrying in {:?}", self.inner.name(), e, d);
                    tokio::time::sleep(d).await;
                    attempt += 1;
                }
                other => return other,
            }
        }
    }
}

#[async_trait::async_trait]
impl LlmProvider for Retrying {
    fn name(&self) -> &str {
        self.inner.name()
    }

    async fn chat(&self, req: &ChatRequest, on_delta: OnDelta<'_>) -> ProviderResult<ChatResponse> {
        let streamed = AtomicBool::new(false);
        let forward = |d: &str| {
            streamed.store(true, Ordering::SeqCst);
            if let Some(f) = on_delta {
                f(d);
            }
        };
        let mut attempt = 0;
        loop {
            let cb: OnDelta<'_> = if on_delta.is_some() {
                Some(&forward)
            } else {
                None
            };
            match self.inner.chat(req, cb).await {
                Err(e)
                    if e.transient()
                        && attempt + 1 < self.attempts
                        && !streamed.load(Ordering::SeqCst) =>
                {
                    let Some(d) = self.delay(attempt, e.retry_after_s) else {
                        return Err(e);
                    };
                    tracing::info!("{}: {} — retrying in {:?}", self.inner.name(), e, d);
                    tokio::time::sleep(d).await;
                    attempt += 1;
                }
                other => return other,
            }
        }
    }

    async fn transcribe(
        &self,
        model: &str,
        audio: Vec<u8>,
        file_name: &str,
        language: Option<&str>,
    ) -> ProviderResult<String> {
        self.run(|| {
            self.inner
                .transcribe(model, audio.clone(), file_name, language)
        })
        .await
    }

    async fn speech(&self, model: &str, voice: &str, text: &str) -> ProviderResult<Vec<u8>> {
        self.run(|| self.inner.speech(model, voice, text)).await
    }

    async fn embed(&self, model: &str, inputs: &[String]) -> ProviderResult<Vec<Vec<f32>>> {
        self.run(|| self.inner.embed(model, inputs)).await
    }

    async fn list_models(&self) -> ProviderResult<Vec<String>> {
        self.run(|| self.inner.list_models()).await
    }
}

pub fn wrap(p: DynProvider) -> DynProvider {
    Arc::new(Retrying::new(p))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::ledger::Usage;
    use crate::providers::{ProviderError, ProviderErrorKind, StopReason};

    /// Fails with the given errors in order, then succeeds.
    struct Flaky {
        errors: Mutex<Vec<ProviderError>>,
        calls: Mutex<u32>,
        stream_before_fail: bool,
    }

    #[async_trait::async_trait]
    impl LlmProvider for Flaky {
        fn name(&self) -> &str {
            "Flaky"
        }
        async fn chat(&self, _: &ChatRequest, d: OnDelta<'_>) -> ProviderResult<ChatResponse> {
            *self.calls.lock().unwrap() += 1;
            if let Some(e) = self.errors.lock().unwrap().pop() {
                if self.stream_before_fail
                    && let Some(f) = d
                {
                    f("partial");
                }
                return Err(e);
            }
            Ok(ChatResponse {
                text: "ok".into(),
                tool_calls: vec![],
                usage: Usage::default(),
                stop: StopReason::EndTurn,
            })
        }
    }

    fn req() -> ChatRequest {
        ChatRequest {
            model: "m".into(),
            system: String::new(),
            messages: vec![],
            tools: vec![],
            max_tokens: 1,
            temperature: None,
            json: false,
            params: Default::default(),
        }
    }

    fn flaky(errors: Vec<ProviderError>, stream_before_fail: bool) -> Arc<Flaky> {
        Arc::new(Flaky {
            errors: Mutex::new(errors),
            calls: Mutex::new(0),
            stream_before_fail,
        })
    }

    #[tokio::test]
    async fn retries_transient_then_succeeds() {
        let f = flaky(
            vec![
                ProviderError::new(ProviderErrorKind::Server, "5xx"),
                ProviderError::new(ProviderErrorKind::RateLimited, "429"),
            ],
            false,
        );
        let r = Retrying::fast(f.clone());
        assert_eq!(r.chat(&req(), None).await.unwrap().text, "ok");
        assert_eq!(*f.calls.lock().unwrap(), 3);
    }

    #[tokio::test]
    async fn permanent_errors_and_started_streams_are_not_retried() {
        let f = flaky(
            vec![ProviderError::new(ProviderErrorKind::Auth, "key")],
            false,
        );
        assert!(Retrying::fast(f.clone()).chat(&req(), None).await.is_err());
        assert_eq!(*f.calls.lock().unwrap(), 1);

        let f = flaky(
            vec![ProviderError::new(ProviderErrorKind::Network, "drop")],
            true,
        );
        let sink = |_: &str| {};
        assert!(
            Retrying::fast(f.clone())
                .chat(&req(), Some(&sink))
                .await
                .is_err()
        );
        assert_eq!(*f.calls.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn long_retry_after_goes_back_to_the_queue() {
        let mut e = ProviderError::new(ProviderErrorKind::RateLimited, "429");
        e.retry_after_s = Some(3600);
        let f = flaky(vec![e], false);
        assert!(Retrying::fast(f.clone()).chat(&req(), None).await.is_err());
        assert_eq!(*f.calls.lock().unwrap(), 1);
    }
}
