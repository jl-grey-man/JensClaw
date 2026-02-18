use sandy::error_classifier::{classify_io_error, ErrorClass};
use std::io;

fn main() {
    println!("=== Testing Error Classifier ===\n");

    // Test 1: Timeout (should be Recoverable)
    println!("Test 1: IO Timeout");
    let err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
    let class = classify_io_error(&err);
    println!("  Result: {:?}", class);
    println!("  Expected: Recoverable");
    println!("  ✅ {}\n", if class == ErrorClass::Recoverable { "PASS" } else { "FAIL" });

    // Test 2: Permission Denied (should be Auth)
    println!("Test 2: Permission Denied");
    let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let class = classify_io_error(&err);
    println!("  Result: {:?}", class);
    println!("  Expected: Auth");
    println!("  ✅ {}\n", if class == ErrorClass::Auth { "PASS" } else { "FAIL" });

    // Test 3: Not Found (should be Permanent)
    println!("Test 3: Not Found");
    let err = io::Error::new(io::ErrorKind::NotFound, "not found");
    let class = classify_io_error(&err);
    println!("  Result: {:?}", class);
    println!("  Expected: Permanent");
    println!("  ✅ {}\n", if class == ErrorClass::Permanent { "PASS" } else { "FAIL" });

    // Test 4: Connection Reset (should be Recoverable)
    println!("Test 4: Connection Reset");
    let err = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
    let class = classify_io_error(&err);
    println!("  Result: {:?}", class);
    println!("  Expected: Recoverable");
    println!("  ✅ {}\n", if class == ErrorClass::Recoverable { "PASS" } else { "FAIL" });

    // Test 5: Connection Refused (should be Recoverable)
    println!("Test 5: Connection Refused");
    let err = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
    let class = classify_io_error(&err);
    println!("  Result: {:?}", class);
    println!("  Expected: Recoverable");
    println!("  ✅ {}\n", if class == ErrorClass::Recoverable { "PASS" } else { "FAIL" });

    println!("=== All Tests Complete ===");
}
