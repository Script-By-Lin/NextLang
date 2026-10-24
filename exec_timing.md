## Execution Timing and Runtime Statistics

Implement accurate execution-time reporting for NextLang's existing CLI execution command.

### Requirements

1. Use Rust's `std::time::Instant` for monotonic elapsed-time measurement.
2. Start the execution timer immediately before program execution and stop it immediately after execution finishes, including when the program returns a runtime error.
3. Display the execution duration after the program's output, whether execution succeeds or fails.
4. Format durations appropriately:
   - Nanoseconds or microseconds for very short executions.
   - Milliseconds for ordinary executions.
   - Seconds for longer executions.
5. Clearly distinguish execution time from compilation time, startup time, and total command duration.
6. If NextLang interprets source directly, label the metric `Execution time`.
7. If NextLang later compiles source before execution, measure compilation and execution separately when both measurements are available.
8. Print timing information to stderr by default so scripts can continue to consume the program's stdout without unexpected extra text.
9. Provide a CLI option such as `--time` if appropriate, allowing users to enable or disable timing output. Choose behavior consistent with the existing CLI conventions.
10. Ensure timing information is displayed for successful execution, runtime errors, and non-zero program exit statuses where applicable.
11. Do not fabricate memory statistics. Only report peak memory if it is actually measured using a documented method.
12. Add automated tests for timing output, runtime-error handling, and stdout/stderr separation. Avoid brittle tests that require a particular execution duration.
13. Update CLI documentation and examples.
14. Preserve existing commands and functionality.

Run the workspace tests, formatting checks, and Clippy. Report actual results and document any limitations.

Do not begin another development phase as part of this change.