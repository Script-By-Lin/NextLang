use crate::error::RuntimeError;
use crate::heap::HeapHandle;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ObjectInstance {
    pub class_name: String,
    pub fields: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub enum Value {
    Int(i128),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Set(Rc<RefCell<HashSet<String>>>),
    Object(Rc<RefCell<ObjectInstance>>),
    Super(Rc<RefCell<ObjectInstance>>, String),
    Struct {
        name: String,
        fields: Rc<RefCell<HashMap<String, Value>>>,
    },
    Heap(HeapHandle),
    Reference(Rc<RefCell<Value>>),
    Future(Rc<RefCell<TaskHandle>>),
    Null,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FutureState {
    Pending,
    Completed(Box<Value>),
    Failed(String),
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct TaskHandle {
    pub id: u64,
    pub state: FutureState,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Null | Value::Void => false,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "INT",
            Value::Float(_) => "FL",
            Value::Bool(_) => "BOOL",
            Value::Char(_) => "CHAR",
            Value::String(_) => "STRING",
            Value::List(_) => "LIST",
            Value::Map(_) => "MAP",
            Value::Set(_) => "SET",
            Value::Object(_) | Value::Super(_, _) => "OBJECT",
            Value::Struct { .. } => "STRUCT",
            Value::Heap(_) => "HEAP",
            Value::Reference(_) => "REFERENCE",
            Value::Future(_) => "FUTURE",
            Value::Null => "NULL",
            Value::Void => "VOID",
        }
    }

    pub fn unwrap_ref(&self) -> Value {
        match self {
            Value::Reference(r) => r.borrow().clone().unwrap_ref(),
            other => other.clone(),
        }
    }

    pub fn add(&self, rhs: &Value) -> Result<Value, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a + b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a + b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 + b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a + *b as f64)),
            (Value::String(a), b) => Ok(Value::String(format!("{}{}", a, b))),
            (a, Value::String(b)) => Ok(Value::String(format!("{}{}", a, b))),
            _ => Err(RuntimeError::new(format!(
                "cannot add {} and {}",
                left.type_name(),
                right.type_name()
            ))),
        }
    }

    pub fn sub(&self, rhs: &Value) -> Result<Value, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a - b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a - b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 - b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a - *b as f64)),
            _ => Err(RuntimeError::new(format!(
                "cannot subtract {} from {}",
                right.type_name(),
                left.type_name()
            ))),
        }
    }

    pub fn mul(&self, rhs: &Value) -> Result<Value, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a * b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a * b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 * b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a * *b as f64)),
            (Value::String(s), Value::Int(n)) | (Value::Int(n), Value::String(s)) => {
                let times = (*n).max(0) as usize;
                Ok(Value::String(s.repeat(times)))
            }
            _ => Err(RuntimeError::new(format!(
                "cannot multiply {} and {}",
                left.type_name(),
                right.type_name()
            ))),
        }
    }

    pub fn div(&self, rhs: &Value) -> Result<Value, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(_), Value::Int(0)) => Err(RuntimeError::new("division by zero")),
            (Value::Float(_), Value::Float(b)) if *b == 0.0 => {
                Err(RuntimeError::new("division by zero"))
            }
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a / b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a / b)),
            (Value::Int(a), Value::Float(b)) => Ok(Value::Float(*a as f64 / b)),
            (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a / *b as f64)),
            _ => Err(RuntimeError::new(format!(
                "cannot divide {} by {}",
                left.type_name(),
                right.type_name()
            ))),
        }
    }

    pub fn rem(&self, rhs: &Value) -> Result<Value, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(_), Value::Int(0)) => Err(RuntimeError::new("modulo by zero")),
            (Value::Float(_), Value::Float(b)) if *b == 0.0 => {
                Err(RuntimeError::new("modulo by zero"))
            }
            (Value::Int(a), Value::Int(b)) => Ok(Value::Int(a % b)),
            (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a % b)),
            _ => Err(RuntimeError::new(format!(
                "cannot modulo {} by {}",
                left.type_name(),
                right.type_name()
            ))),
        }
    }

    pub fn equals(&self, rhs: &Value) -> bool {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => (a - b).abs() < f64::EPSILON,
            (Value::Int(a), Value::Float(b)) => (*a as f64 - b).abs() < f64::EPSILON,
            (Value::Float(a), Value::Int(b)) => (a - *b as f64).abs() < f64::EPSILON,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Char(a), Value::Char(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Heap(a), Value::Heap(b)) => a == b,
            (Value::Null, Value::Null) => true,
            (Value::Void, Value::Void) => true,
            (Value::List(a), Value::List(b)) => {
                let a = a.borrow();
                let b = b.borrow();
                if a.len() != b.len() {
                    return false;
                }
                a.iter().zip(b.iter()).all(|(x, y)| x.equals(y))
            }
            (Value::Object(a), Value::Object(b)) => Rc::ptr_eq(a, b),
            (Value::Super(a, _), Value::Super(b, _)) => Rc::ptr_eq(a, b),
            (Value::Object(a), Value::Super(b, _)) | (Value::Super(a, _), Value::Object(b)) => {
                Rc::ptr_eq(a, b)
            }
            _ => false,
        }
    }

    pub fn compare(&self, rhs: &Value) -> Result<std::cmp::Ordering, RuntimeError> {
        let left = self.unwrap_ref();
        let right = rhs.unwrap_ref();
        match (&left, &right) {
            (Value::Int(a), Value::Int(b)) => Ok(a.cmp(b)),
            (Value::Float(a), Value::Float(b)) => a
                .partial_cmp(b)
                .ok_or_else(|| RuntimeError::new("cannot compare NaN")),
            (Value::Int(a), Value::Float(b)) => (*a as f64)
                .partial_cmp(b)
                .ok_or_else(|| RuntimeError::new("cannot compare NaN")),
            (Value::Float(a), Value::Int(b)) => a
                .partial_cmp(&(*b as f64))
                .ok_or_else(|| RuntimeError::new("cannot compare NaN")),
            (Value::String(a), Value::String(b)) => Ok(a.cmp(b)),
            (Value::Char(a), Value::Char(b)) => Ok(a.cmp(b)),
            _ => Err(RuntimeError::new(format!(
                "cannot compare {} and {}",
                left.type_name(),
                right.type_name()
            ))),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Bool(b) => write!(f, "{}", b),
            Value::Char(c) => write!(f, "{}", c),
            Value::String(s) => write!(f, "{}", s),
            Value::List(l) => {
                write!(f, "[")?;
                let items = l.borrow();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                let entries = m.borrow();
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Set(s) => {
                write!(f, "{{")?;
                let items = s.borrow();
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\"", item)?;
                }
                write!(f, "}}")
            }
            Value::Object(obj) | Value::Super(obj, _) => {
                let inst = obj.borrow();
                write!(f, "<instance of {}>", inst.class_name)
            }
            Value::Struct { name, fields } => {
                write!(f, "{} {{", name)?;
                for (i, (k, v)) in fields.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Heap(h) => write!(f, "<heap slot={} gen={}>", h.slot, h.generation),
            Value::Reference(r) => write!(f, "&{}", r.borrow()),
            Value::Future(h) => write!(f, "<future #{}>", h.borrow().id),
            Value::Null => write!(f, "null"),
            Value::Void => Ok(()),
        }
    }
}
