use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::{debug, warn, error, info};

use crate::backoff::BackoffPolicy;
use crate::claude::{Message, MessagesResponse, ToolDefinition};
use crate::error::MicroClawError;
use crate::error_classifier::{classify_http_error, ErrorClass};
use crate::llm::LlmProvider;

/// Tracks cooldown state for a single model.
#[derive(Debug, Clone)]
struct CooldownState {
    consecutive_failures: u32,
    cooldown_until: Option<Instant>,
}

impl CooldownState {
    fn new() -> Self {
        Self {
            consecutive_failures: 0,
            cooldown_until: None,
        }
    }

    fn record_failure(&mut self) {
        self.consecutive_failures += 1;
        // Cooldown: min(30 * 2^failures, 600) seconds
        let secs = (30u64 * 2u64.pow(self.consecutive_failures.min(10))).min(600);
        self.cooldown_until = Some(Instant::now() + Duration::from_secs(secs));
    }

    fn record_success(&mut self) {
        self.consecutive_failures = 0;
        self.cooldown_until = None;
    }

    fn is_cooling_down(&self) -> bool {
        self.cooldown_until
            .map(|until| Instant::now() < until)
            .unwrap_or(false)
    }

    fn cooldown_remaining(&self) -> Option<Duration> {
        self.cooldown_until.and_then(|until| {
            let now = Instant::now();
            if now < until {
                Some(until - now)
            } else {
                None
            }
        })
    }
}

/// Tracks cooldown state across multiple models.
pub struct ModelCooldown {
    states: Mutex<HashMap<String, CooldownState>>,
}

impl ModelCooldown {
    pub fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
        }
    }

    pub fn record_failure(&self, model: &str) {
        let mut states = self.states.lock().unwrap();
        let state = states.entry(model.to_string()).or_insert_with(CooldownState::new);
        state.record_failure();
        info!(
            "Model '{}' failure #{}, cooling down for {:?}",
            model,
            state.consecutive_failures,
            state.cooldown_remaining()
        );
    }

    pub fn record_success(&self, model: &str) {
        let mut states = self.states.lock().unwrap();
        if let Some(state) = states.get_mut(model) {
            state.record_success();
        }
    }

    pub fn is_cooling_down(&self, model: &str) -> bool {
        let states = self.states.lock().unwrap();
        states
            .get(model)
            .map(|s| s.is_cooling_down())
            .unwrap_or(false)
    }

    /// Select the best available model from a list, skipping models in cooldown.
    /// Returns None if all models are cooling down.
    pub fn select_available<'a>(&self, models: &'a [String]) -> Option<&'a str> {
        let states = self.states.lock().unwrap();
        for model in models {
            let cooling = states
                .get(model.as_str())
                .map(|s| s.is_cooling_down())
                .unwrap_or(false);
            if !cooling {
                return Some(model.as_str());
            }
        }
        // All cooling down - return the first one (least bad option)
        models.first().map(|s| s.as_str())
    }
}

impl Default for ModelCooldown {
    fn default() -> Self {
        Self::new()
    }
}

/// Wraps an LlmProvider and adds retry logic with exponential backoff
pub struct RetryLlmProvider {
    inner: Box<dyn LlmProvider>,
    max_attempts: u32,
    pub cooldown: ModelCooldown,
}

impl RetryLlmProvider {
    pub fn new(inner: Box<dyn LlmProvider>) -> Self {
        Self {
            inner,
            max_attempts: 5, // Maximum 5 attempts
            cooldown: ModelCooldown::new(),
        }
    }

    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }
}

#[async_trait]
impl LlmProvider for RetryLlmProvider {
    async fn send_message(
        &self,
        system: &str,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
    ) -> Result<MessagesResponse, MicroClawError> {
        let mut attempt = 0;
        let policy = BackoffPolicy::default_network();

        loop {
            attempt += 1;

            debug!("LLM call attempt {}/{}", attempt, self.max_attempts);

            match self.inner.send_message(system, messages.clone(), tools.clone()).await {
                Ok(response) => {
                    if attempt > 1 {
                        debug!("LLM call succeeded on attempt {}", attempt);
                    }
                    return Ok(response);
                }
                Err(error) => {
                    // Classify the error to determine retry strategy
                    let error_class = match &error {
                        MicroClawError::Http(e) => classify_http_error(e),
                        MicroClawError::RateLimited => ErrorClass::RateLimit,
                        _ => {
                            // For other errors, be conservative and don't retry
                            ErrorClass::Permanent
                        }
                    };

                    match error_class {
                        ErrorClass::Permanent => {
                            error!("Permanent error, not retrying: {}", error);
                            return Err(error);
                        }
                        ErrorClass::Auth => {
                            error!("Auth error, not retrying yet: {}", error);
                            return Err(error);
                        }
                        ErrorClass::Recoverable | ErrorClass::RateLimit => {
                            if attempt >= self.max_attempts {
                                error!("Max attempts ({}) reached, giving up: {}", self.max_attempts, error);
                                return Err(error);
                            }

                            let wait = policy.compute(attempt);
                            warn!(
                                "Attempt {} failed ({:?}), retrying in {:?}: {}",
                                attempt,
                                error_class,
                                wait,
                                error
                            );

                            tokio::time::sleep(wait).await;
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cooldown_state_initial() {
        let state = CooldownState::new();
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.is_cooling_down());
    }

    #[test]
    fn test_cooldown_state_failure() {
        let mut state = CooldownState::new();
        state.record_failure();
        assert_eq!(state.consecutive_failures, 1);
        assert!(state.is_cooling_down());
        assert!(state.cooldown_remaining().is_some());
    }

    #[test]
    fn test_cooldown_state_success_resets() {
        let mut state = CooldownState::new();
        state.record_failure();
        state.record_failure();
        assert_eq!(state.consecutive_failures, 2);
        state.record_success();
        assert_eq!(state.consecutive_failures, 0);
        assert!(!state.is_cooling_down());
    }

    #[test]
    fn test_cooldown_duration_exponential() {
        let mut state = CooldownState::new();

        // First failure: 30 * 2^1 = 60s
        state.record_failure();
        let remaining = state.cooldown_remaining().unwrap();
        assert!(remaining.as_secs() <= 60);
        assert!(remaining.as_secs() >= 55); // allow some timing slack

        // Reset and do 2 failures
        state = CooldownState::new();
        state.record_failure();
        state.record_failure();
        // 2nd failure: 30 * 2^2 = 120s
        let remaining = state.cooldown_remaining().unwrap();
        assert!(remaining.as_secs() <= 120);
        assert!(remaining.as_secs() >= 115);
    }

    #[test]
    fn test_cooldown_duration_capped_at_600() {
        let mut state = CooldownState::new();
        for _ in 0..20 {
            state.record_failure();
        }
        let remaining = state.cooldown_remaining().unwrap();
        assert!(remaining.as_secs() <= 600);
    }

    #[test]
    fn test_model_cooldown_multiple_models() {
        let cooldown = ModelCooldown::new();

        assert!(!cooldown.is_cooling_down("model-a"));
        assert!(!cooldown.is_cooling_down("model-b"));

        cooldown.record_failure("model-a");
        assert!(cooldown.is_cooling_down("model-a"));
        assert!(!cooldown.is_cooling_down("model-b"));

        cooldown.record_success("model-a");
        assert!(!cooldown.is_cooling_down("model-a"));
    }

    #[test]
    fn test_select_available_skips_cooling() {
        let cooldown = ModelCooldown::new();
        let models = vec!["primary".into(), "fallback-1".into(), "fallback-2".into()];

        // All available -> returns primary
        assert_eq!(cooldown.select_available(&models), Some("primary"));

        // Primary cooling -> returns fallback-1
        cooldown.record_failure("primary");
        assert_eq!(cooldown.select_available(&models), Some("fallback-1"));

        // Primary + fallback-1 cooling -> returns fallback-2
        cooldown.record_failure("fallback-1");
        assert_eq!(cooldown.select_available(&models), Some("fallback-2"));

        // All cooling -> returns primary (least bad)
        cooldown.record_failure("fallback-2");
        assert_eq!(cooldown.select_available(&models), Some("primary"));
    }

    #[test]
    fn test_select_available_empty_list() {
        let cooldown = ModelCooldown::new();
        let models: Vec<String> = vec![];
        assert_eq!(cooldown.select_available(&models), None);
    }
}
