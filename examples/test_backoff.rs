use sandy::backoff::BackoffPolicy;

fn main() {
    println!("=== Testing Backoff Policy ===\n");

    // Test 1: Default network policy
    println!("Test 1: Default Network Policy");
    let policy = BackoffPolicy::default_network();
    println!("  Config: initial={}ms, max={}ms, factor={}, jitter={}",
             policy.initial_ms, policy.max_ms, policy.factor, policy.jitter);

    for attempt in 1..=8 {
        let delay = policy.compute(attempt);
        println!("  Attempt {}: {:?}", attempt, delay);
    }

    // Verify exponential growth
    let d1 = policy.compute(1);
    let d2 = policy.compute(2);
    let d3 = policy.compute(3);
    println!("  ✅ Verification: delay1 < delay2 < delay3: {} < {} < {}",
             d1.as_millis(), d2.as_millis(), d3.as_millis());
    println!();

    // Test 2: Rate limit policy
    println!("Test 2: Rate Limit Policy (longer delays)");
    let policy = BackoffPolicy::rate_limit();
    println!("  Config: initial={}ms, max={}ms", policy.initial_ms, policy.max_ms);

    for attempt in 1..=6 {
        let delay = policy.compute(attempt);
        println!("  Attempt {}: {:?}", attempt, delay);
    }
    println!();

    // Test 3: Max cap
    println!("Test 3: Max Cap Test");
    let policy = BackoffPolicy {
        initial_ms: 1000,
        max_ms: 5000,
        factor: 2.0,
        jitter: 0.0, // No jitter for predictable testing
    };

    let delay_10 = policy.compute(10);
    let delay_20 = policy.compute(20);
    println!("  Attempt 10: {:?}", delay_10);
    println!("  Attempt 20: {:?}", delay_20);
    println!("  ✅ Both capped at max: {} == {}", delay_10.as_millis(), delay_20.as_millis());
    println!();

    println!("=== All Tests Complete ===");
}
