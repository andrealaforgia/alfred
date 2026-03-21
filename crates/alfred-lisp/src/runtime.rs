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
}
