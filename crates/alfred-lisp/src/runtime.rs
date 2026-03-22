//! LispRuntime -- wraps the rust_lisp interpreter for Alfred.
//!
//! Provides a clean `eval(source) -> Result<LispValue, LispError>` API
//! over the underlying Lisp interpreter.

use std::cell::RefCell;
use std::rc::Rc;

use rust_lisp::default_env;
use rust_lisp::interpreter::eval_block;
use rust_lisp::model::{Env, RuntimeError, Value};
use rust_lisp::parser::{parse, ParseError};

/// Error type for Lisp evaluation failures.
///
/// Wraps both parse-time and runtime errors from the underlying
/// rust_lisp interpreter into a single error type.
#[derive(Debug, thiserror::Error)]
pub enum LispError {
    /// A syntax error in the Lisp source code.
    #[error("Parse error: {message}")]
    ParseError { message: String },

    /// A runtime error during evaluation (undefined symbol, type mismatch, etc.).
    #[error("Runtime error: {message}")]
    RuntimeError { message: String },

    /// An I/O error (e.g., file not found for eval_file).
    #[error("IO error: {message}")]
    IoError { message: String },
}

impl From<ParseError> for LispError {
    fn from(error: ParseError) -> Self {
        LispError::ParseError { message: error.msg }
    }
}

impl From<RuntimeError> for LispError {
    fn from(error: RuntimeError) -> Self {
        LispError::RuntimeError { message: error.msg }
    }
}

/// A Lisp value returned from evaluation.
///
/// Wraps the underlying `rust_lisp::Value` and provides typed
/// accessor methods for safe extraction.
#[derive(Debug, Clone)]
pub struct LispValue {
    inner: Value,
}

impl LispValue {
    /// Extract an integer value, or `None` if this is not an integer.
    pub fn as_integer(&self) -> Option<i32> {
        match &self.inner {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// Extract a string value, or `None` if this is not a string.
    pub fn as_string(&self) -> Option<String> {
        match &self.inner {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    /// Access the underlying rust_lisp Value.
    pub fn inner(&self) -> &Value {
        &self.inner
    }
}

impl std::fmt::Display for LispValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<Value> for LispValue {
    fn from(value: Value) -> Self {
        LispValue { inner: value }
    }
}

/// The Lisp runtime wrapping the rust_lisp interpreter.
///
/// Holds the interpreter environment (variable bindings, built-in
/// functions) and provides `eval` as the single driving port.
pub struct LispRuntime {
    env: Rc<RefCell<Env>>,
}

impl Default for LispRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl LispRuntime {
    /// Create a new runtime with the default environment
    /// (arithmetic, list operations, comparisons, etc.).
    pub fn new() -> Self {
        LispRuntime {
            env: Rc::new(RefCell::new(default_env())),
        }
    }

    /// Returns a reference to the underlying environment.
    ///
    /// Used by the bridge module to register native closures.
    pub fn env(&self) -> Rc<RefCell<Env>> {
        self.env.clone()
    }

    /// Read a file and evaluate its contents as Lisp source.
    ///
    /// Loads the file at `path`, parses and evaluates all expressions,
    /// and returns the result of the last expression.
    ///
    /// Returns `Err(LispError::IoError)` if the file cannot be read.
    pub fn eval_file(&self, path: &std::path::Path) -> Result<LispValue, LispError> {
        let source = std::fs::read_to_string(path).map_err(|err| LispError::IoError {
            message: format!("{}: {}", path.display(), err),
        })?;
        self.eval(&source)
    }

    /// Parse and evaluate a Lisp source string.
    ///
    /// If the source contains multiple expressions, all are evaluated
    /// in order and the result of the last expression is returned.
    ///
    /// Returns `Err(LispError)` for syntax errors or runtime errors.
    pub fn eval(&self, source: &str) -> Result<LispValue, LispError> {
        let parsed_expressions = parse(source);

        // Collect parsed values, short-circuiting on parse errors.
        let values: Vec<Value> = parsed_expressions
            .map(|result| result.map_err(LispError::from))
            .collect::<Result<Vec<Value>, LispError>>()?;

        if values.is_empty() {
            return Err(LispError::ParseError {
                message: "Empty expression".to_string(),
            });
        }

        let result = eval_block(self.env.clone(), values.into_iter())?;
        Ok(LispValue::from(result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Acceptance: basic arithmetic through the driving port --

    #[test]
    fn eval_arithmetic_addition_returns_correct_integer() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("(+ 1 2)");

        assert!(result.is_ok(), "eval should succeed for valid arithmetic");
        assert_eq!(result.unwrap().as_integer(), Some(3));
    }

    #[test]
    fn eval_arithmetic_multiplication_returns_correct_integer() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("(* 3 4)").unwrap();

        assert_eq!(result.as_integer(), Some(12));
    }

    // -- String concatenation --

    #[test]
    fn eval_string_concatenation_returns_combined_string() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("(+ \"hello\" \" world\")").unwrap();

        assert_eq!(result.as_string(), Some("hello world".to_string()));
    }

    // -- Variable binding with define --

    #[test]
    fn eval_define_then_reference_returns_bound_value() {
        let runtime = LispRuntime::new();
        runtime.eval("(define x 42)").unwrap();
        let result = runtime.eval("x").unwrap();

        assert_eq!(result.as_integer(), Some(42));
    }

    // -- Error handling: syntax errors --

    #[test]
    fn eval_syntax_error_returns_err_not_panic() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("(+ 1");

        assert!(result.is_err(), "eval should return Err for syntax errors");
        let error = result.unwrap_err();
        assert!(
            error.to_string().contains("parse")
                || error.to_string().contains("Parse")
                || error.to_string().contains("Unclosed"),
            "error message should indicate a parse problem, got: {}",
            error
        );
    }

    // -- Error handling: undefined symbol --

    #[test]
    fn eval_undefined_symbol_returns_err() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("undefined_var");

        assert!(
            result.is_err(),
            "eval should return Err for undefined symbols"
        );
    }

    // -- LispValue accessors --

    #[test]
    fn lisp_value_as_integer_returns_none_for_non_integer() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("\"hello\"").unwrap();

        assert_eq!(result.as_integer(), None);
    }

    #[test]
    fn lisp_value_as_string_returns_none_for_non_string() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("42").unwrap();

        assert_eq!(result.as_string(), None);
    }

    // -- Multiple expressions: last value is returned --

    #[test]
    fn eval_multiple_expressions_returns_last_result() {
        let runtime = LispRuntime::new();
        let result = runtime.eval("(define a 10) (+ a 5)").unwrap();

        assert_eq!(result.as_integer(), Some(15));
    }

    // -- eval_file: load and evaluate a .lisp file --

    #[test]
    fn eval_file_evaluates_file_contents_and_returns_last_result() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.lisp");
        {
            let mut f = std::fs::File::create(&file_path).unwrap();
            writeln!(f, "(define x 10)").unwrap();
            writeln!(f, "(+ x 5)").unwrap();
        }

        let runtime = LispRuntime::new();
        let result = runtime.eval_file(&file_path).unwrap();

        assert_eq!(result.as_integer(), Some(15));
    }

    #[test]
    fn eval_file_returns_error_for_nonexistent_file() {
        let runtime = LispRuntime::new();
        let result = runtime.eval_file(std::path::Path::new("/nonexistent/file.lisp"));

        assert!(result.is_err());
    }
}

/// Performance baseline tests measuring single Lisp primitive eval latency.
///
/// Kill signal threshold: 1ms per call. If any primitive exceeds this,
/// evaluate Janet as an alternative interpreter.
///
/// Run with `cargo test --package alfred-lisp --features perf-tests -- --nocapture perf_baseline`
/// to see timing output.
#[cfg(all(test, feature = "perf-tests"))]
mod perf_baseline {
    use super::*;
    use crate::bridge;
    use alfred_core::cursor;
    use alfred_core::editor_state;
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::time::{Duration, Instant};

    const KILL_SIGNAL_THRESHOLD: Duration = Duration::from_millis(1);
    const WARMUP_ITERATIONS: usize = 10;
    const MEASUREMENT_ITERATIONS: usize = 100;

    /// Creates a runtime with bridge primitives registered and a multi-line buffer.
    fn create_benchmarkable_runtime() -> (LispRuntime, Rc<RefCell<editor_state::EditorState>>) {
        let state = Rc::new(RefCell::new(editor_state::new(80, 24)));
        {
            let mut editor = state.borrow_mut();
            editor.buffer =
                alfred_core::buffer::Buffer::from_string("Line 1\nLine 2\nLine 3\nLine 4\nLine 5");
            editor.cursor = cursor::new(0, 0);
        }
        let runtime = LispRuntime::new();
        bridge::register_core_primitives(&runtime, state.clone());
        (runtime, state)
    }

    /// Measures the median latency of a single eval call over multiple iterations.
    ///
    /// Runs warmup iterations first (discarded), then measures and returns
    /// the median duration from the measurement iterations.
    fn measure_eval_latency(runtime: &LispRuntime, expression: &str) -> Duration {
        // Warmup: let JIT / caches settle
        for _ in 0..WARMUP_ITERATIONS {
            let _ = runtime.eval(expression);
        }

        // Measure
        let mut durations: Vec<Duration> = Vec::with_capacity(MEASUREMENT_ITERATIONS);
        for _ in 0..MEASUREMENT_ITERATIONS {
            let start = Instant::now();
            let _ = runtime.eval(expression);
            durations.push(start.elapsed());
        }

        durations.sort();
        durations[MEASUREMENT_ITERATIONS / 2]
    }

    // -----------------------------------------------------------------------
    // Acceptance: all core primitives complete under 1ms kill signal threshold
    // -----------------------------------------------------------------------

    #[test]
    fn all_core_primitives_eval_under_kill_signal_threshold() {
        let (runtime, _state) = create_benchmarkable_runtime();

        let primitives = [
            ("arithmetic (+ 1 2)", "(+ 1 2)"),
            ("buffer-insert", "(buffer-insert \"x\")"),
            ("cursor-move", "(cursor-move ':down 1)"),
            ("message", "(message \"test\")"),
            ("buffer-content", "(buffer-content)"),
            ("cursor-position", "(cursor-position)"),
            ("current-mode", "(current-mode)"),
        ];

        println!();
        println!("=== Alfred Lisp Performance Baseline (Step 02-05) ===");
        println!(
            "Kill signal threshold: {:?} per call",
            KILL_SIGNAL_THRESHOLD
        );
        println!(
            "Measurement: median of {} iterations ({} warmup)",
            MEASUREMENT_ITERATIONS, WARMUP_ITERATIONS
        );
        println!("---------------------------------------------------");

        let mut all_passed = true;
        let mut results: Vec<(String, Duration, bool)> = Vec::new();

        for (name, expression) in &primitives {
            let median_latency = measure_eval_latency(&runtime, expression);
            let within_threshold = median_latency < KILL_SIGNAL_THRESHOLD;
            let status = if within_threshold {
                "PASS"
            } else {
                "KILL SIGNAL"
            };

            println!("  {:<25} {:>10.2?}  [{}]", name, median_latency, status);

            if !within_threshold {
                all_passed = false;
            }
            results.push((name.to_string(), median_latency, within_threshold));
        }

        println!("---------------------------------------------------");

        if !all_passed {
            let failures: Vec<String> = results
                .iter()
                .filter(|(_, _, passed)| !passed)
                .map(|(name, latency, _)| format!("{}: {:?}", name, latency))
                .collect();

            panic!(
                "KILL SIGNAL: The following primitives exceeded the 1ms threshold.\n\
                 Evaluate Janet as an alternative interpreter.\n\
                 Failures: {}",
                failures.join(", ")
            );
        }

        println!("Result: ALL PRIMITIVES WITHIN THRESHOLD -- rust_lisp is viable.");
        println!("===================================================");
    }

    // -----------------------------------------------------------------------
    // Unit: individual primitive latency measurements
    // -----------------------------------------------------------------------

    #[test]
    fn buffer_insert_eval_latency_under_one_millisecond() {
        let (runtime, _state) = create_benchmarkable_runtime();
        let median_latency = measure_eval_latency(&runtime, "(buffer-insert \"x\")");

        println!(
            "\n[perf] buffer-insert median latency: {:?} (threshold: {:?})",
            median_latency, KILL_SIGNAL_THRESHOLD
        );

        assert!(
            median_latency < KILL_SIGNAL_THRESHOLD,
            "KILL SIGNAL: buffer-insert latency {:?} exceeds 1ms threshold. Evaluate Janet.",
            median_latency
        );
    }

    #[test]
    fn cursor_move_eval_latency_under_one_millisecond() {
        let (runtime, _state) = create_benchmarkable_runtime();
        let median_latency = measure_eval_latency(&runtime, "(cursor-move ':down 1)");

        println!(
            "\n[perf] cursor-move median latency: {:?} (threshold: {:?})",
            median_latency, KILL_SIGNAL_THRESHOLD
        );

        assert!(
            median_latency < KILL_SIGNAL_THRESHOLD,
            "KILL SIGNAL: cursor-move latency {:?} exceeds 1ms threshold. Evaluate Janet.",
            median_latency
        );
    }

    #[test]
    fn arithmetic_eval_latency_under_one_millisecond() {
        let (runtime, _state) = create_benchmarkable_runtime();
        let median_latency = measure_eval_latency(&runtime, "(+ 1 2)");

        println!(
            "\n[perf] arithmetic (+ 1 2) median latency: {:?} (threshold: {:?})",
            median_latency, KILL_SIGNAL_THRESHOLD
        );

        assert!(
            median_latency < KILL_SIGNAL_THRESHOLD,
            "KILL SIGNAL: arithmetic eval latency {:?} exceeds 1ms threshold. Evaluate Janet.",
            median_latency
        );
    }

    #[test]
    fn message_eval_latency_under_one_millisecond() {
        let (runtime, _state) = create_benchmarkable_runtime();
        let median_latency = measure_eval_latency(&runtime, "(message \"hello\")");

        println!(
            "\n[perf] message median latency: {:?} (threshold: {:?})",
            median_latency, KILL_SIGNAL_THRESHOLD
        );

        assert!(
            median_latency < KILL_SIGNAL_THRESHOLD,
            "KILL SIGNAL: message latency {:?} exceeds 1ms threshold. Evaluate Janet.",
            median_latency
        );
    }
}
