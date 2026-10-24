use nx_ast::Type as AstType;
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SemType {
    // Signed integers
    Int8,
    Int16,
    Int32,
    Int64,
    Int128,

    // Unsigned integers
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Uint128,

    // Platform-dependent pointer-sized integers & pointers
    Usize,
    Isize,
    Ptr,

    // Floating point
    Fl32,
    Fl64,
    Fl128,

    // Primitives
    Bool,
    Char,
    String,
    Any,
    Void,

    // Generic collections
    List(Box<SemType>),
    Set(Box<SemType>),
    Map(Box<SemType>, Box<SemType>),

    // Union types
    Union(Vec<SemType>),

    // Reference types (&T, &mut T)
    Reference { inner: Box<SemType>, is_mut: bool },

    // Custom named types
    Custom(String),

    // Error recovery sentinel
    Unknown,
}

impl SemType {
    pub fn from_ast(ast_type: &AstType) -> Self {
        match ast_type {
            AstType::Named { name, .. } => Self::from_str_name(name),
            AstType::Generic {
                name, type_args, ..
            } => match name.as_str() {
                "LIST" => {
                    let elem = type_args
                        .first()
                        .map(Self::from_ast)
                        .unwrap_or(SemType::Unknown);
                    SemType::List(Box::new(elem))
                }
                "SET" => {
                    let elem = type_args
                        .first()
                        .map(Self::from_ast)
                        .unwrap_or(SemType::Unknown);
                    SemType::Set(Box::new(elem))
                }
                "MAP" => {
                    let key = type_args
                        .first()
                        .map(Self::from_ast)
                        .unwrap_or(SemType::Unknown);
                    let val = type_args
                        .get(1)
                        .map(Self::from_ast)
                        .unwrap_or(SemType::Unknown);
                    SemType::Map(Box::new(key), Box::new(val))
                }
                _ => SemType::Custom(name.clone()),
            },
            AstType::Union { types, .. } => {
                let sem_types: Vec<SemType> = types.iter().map(Self::from_ast).collect();
                SemType::Union(sem_types)
            }
            AstType::Reference { inner, is_mut, .. } => SemType::Reference {
                inner: Box::new(Self::from_ast(inner)),
                is_mut: *is_mut,
            },
        }
    }

    pub fn from_str_name(name: &str) -> Self {
        match name {
            "INT8" => SemType::Int8,
            "INT16" => SemType::Int16,
            "INT32" => SemType::Int32,
            "INT64" => SemType::Int64,
            "INT128" => SemType::Int128,

            "UINT8" => SemType::Uint8,
            "UINT16" => SemType::Uint16,
            "UINT32" => SemType::Uint32,
            "UINT64" => SemType::Uint64,
            "UINT128" => SemType::Uint128,

            "USIZE" | "usize" => SemType::Usize,
            "ISIZE" | "isize" => SemType::Isize,
            "PTR" | "ptr" | "RAWPTR" => SemType::Ptr,

            "FL32" => SemType::Fl32,
            "FL64" => SemType::Fl64,
            "FL128" => SemType::Fl128,

            "BOOL" => SemType::Bool,
            "CHAR" => SemType::Char,
            "STRING" => SemType::String,
            "ANY" => SemType::Any,
            "VOID" => SemType::Void,
            _ => SemType::Custom(name.to_string()),
        }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            SemType::Int8
                | SemType::Int16
                | SemType::Int32
                | SemType::Int64
                | SemType::Int128
                | SemType::Uint8
                | SemType::Uint16
                | SemType::Uint32
                | SemType::Uint64
                | SemType::Uint128
                | SemType::Usize
                | SemType::Isize
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, SemType::Fl32 | SemType::Fl64 | SemType::Fl128)
    }

    pub fn is_numeric(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    pub fn is_signed_integer(&self) -> bool {
        matches!(
            self,
            SemType::Int8
                | SemType::Int16
                | SemType::Int32
                | SemType::Int64
                | SemType::Int128
                | SemType::Isize
        )
    }

    pub fn is_unsigned_integer(&self) -> bool {
        matches!(
            self,
            SemType::Uint8
                | SemType::Uint16
                | SemType::Uint32
                | SemType::Uint64
                | SemType::Uint128
                | SemType::Usize
        )
    }

    /// Returns true if this type has Copy semantics (primitives).
    pub fn is_copy(&self) -> bool {
        self.is_numeric() || matches!(self, SemType::Bool | SemType::Char | SemType::Ptr)
    }

    /// Checks if a value of type `self` can be assigned to a target of type `target`.
    pub fn is_assignable_to(&self, target: &SemType) -> bool {
        self.is_assignable_to_with_hierarchy(target, &crate::oop::ClassHierarchy::empty())
    }

    /// Checks assignment compatibility using the class and interface inheritance hierarchy.
    pub fn is_assignable_to_with_hierarchy(
        &self,
        target: &SemType,
        hierarchy: &crate::oop::ClassHierarchy,
    ) -> bool {
        // Unknown type is treated as compatible to prevent error cascades
        if *self == SemType::Unknown || *target == SemType::Unknown {
            return true;
        }

        // ANY accepts any value
        if *target == SemType::Any {
            return true;
        }

        // Direct equality
        if self == target {
            return true;
        }

        // Pointer-sized integer compatibility on 64-bit platform
        if matches!(
            (self, target),
            (SemType::Usize, SemType::Uint64) | (SemType::Uint64, SemType::Usize)
        ) {
            return true;
        }
        if matches!(
            (self, target),
            (SemType::Isize, SemType::Int64) | (SemType::Int64, SemType::Isize)
        ) {
            return true;
        }

        // Subtyping: class inheritance or interface implementation
        if let (SemType::Custom(sub), SemType::Custom(sup)) = (self, target) {
            if hierarchy.is_subtype(sub, sup) {
                return true;
            }
        }

        // Reference compatibility
        if let (
            SemType::Reference {
                inner: src_in,
                is_mut: src_mut,
            },
            SemType::Reference {
                inner: dst_in,
                is_mut: dst_mut,
            },
        ) = (self, target)
        {
            if *dst_mut && !*src_mut {
                return false;
            }
            return src_in.is_assignable_to_with_hierarchy(dst_in, hierarchy);
        }

        // Union target: self can be assigned to target if self is assignable to any member
        if let SemType::Union(members) = target {
            return members
                .iter()
                .any(|m| self.is_assignable_to_with_hierarchy(m, hierarchy));
        }

        // Union source: can only be assigned to non-union target if all members can be assigned
        if let SemType::Union(members) = self {
            return members
                .iter()
                .all(|m| m.is_assignable_to_with_hierarchy(target, hierarchy));
        }

        // Collections covariance
        match (self, target) {
            (SemType::List(src_elem), SemType::List(dst_elem)) => {
                src_elem.is_assignable_to_with_hierarchy(dst_elem, hierarchy)
            }
            (SemType::Set(src_elem), SemType::Set(dst_elem)) => {
                src_elem.is_assignable_to_with_hierarchy(dst_elem, hierarchy)
            }
            (SemType::Map(src_k, src_v), SemType::Map(dst_k, dst_v)) => {
                src_k.is_assignable_to_with_hierarchy(dst_k, hierarchy)
                    && src_v.is_assignable_to_with_hierarchy(dst_v, hierarchy)
            }
            _ => false,
        }
    }

    /// Checks whether an integer literal fits into this type.
    pub fn fits_integer_literal(&self, val: i128) -> bool {
        match self {
            SemType::Int8 => val >= i8::MIN as i128 && val <= i8::MAX as i128,
            SemType::Int16 => val >= i16::MIN as i128 && val <= i16::MAX as i128,
            SemType::Int32 => val >= i32::MIN as i128 && val <= i32::MAX as i128,
            SemType::Int64 => val >= i64::MIN as i128 && val <= i64::MAX as i128,
            SemType::Int128 => true,

            SemType::Uint8 => val >= 0 && val <= u8::MAX as i128,
            SemType::Uint16 => val >= 0 && val <= u16::MAX as i128,
            SemType::Uint32 => val >= 0 && val <= u32::MAX as i128,
            SemType::Uint64 => val >= 0 && (val as u128) <= u64::MAX as u128,
            SemType::Uint128 => val >= 0,
            SemType::Usize => val >= 0 && (val as u128) <= u64::MAX as u128,
            SemType::Isize => val >= i64::MIN as i128 && val <= i64::MAX as i128,
            SemType::Ptr => val >= 0 && (val as u128) <= u64::MAX as u128,

            // Floats also accept integer literals in initializers
            SemType::Fl32 | SemType::Fl64 | SemType::Fl128 => true,
            SemType::Any => true,
            SemType::Union(members) => members.iter().any(|m| m.fits_integer_literal(val)),
            _ => false,
        }
    }

    pub fn integer_range_str(&self) -> &'static str {
        match self {
            SemType::Int8 => "-128..127",
            SemType::Int16 => "-32768..32767",
            SemType::Int32 => "-2147483648..2147483647",
            SemType::Int64 => "-9223372036854775808..9223372036854775807",
            SemType::Int128 => {
                "-170141183460469231731687303715884105728..170141183460469231731687303715884105727"
            }
            SemType::Uint8 => "0..255",
            SemType::Uint16 => "0..65535",
            SemType::Uint32 => "0..4294967295",
            SemType::Uint64 => "0..18446744073709551615",
            SemType::Uint128 => "0..340282366920938463463374607431768211455",
            _ => "unknown",
        }
    }
}

impl fmt::Display for SemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SemType::Int8 => write!(f, "INT8"),
            SemType::Int16 => write!(f, "INT16"),
            SemType::Int32 => write!(f, "INT32"),
            SemType::Int64 => write!(f, "INT64"),
            SemType::Int128 => write!(f, "INT128"),
            SemType::Uint8 => write!(f, "UINT8"),
            SemType::Uint16 => write!(f, "UINT16"),
            SemType::Uint32 => write!(f, "UINT32"),
            SemType::Uint64 => write!(f, "UINT64"),
            SemType::Uint128 => write!(f, "UINT128"),
            SemType::Usize => write!(f, "USIZE"),
            SemType::Isize => write!(f, "ISIZE"),
            SemType::Ptr => write!(f, "PTR"),
            SemType::Fl32 => write!(f, "FL32"),
            SemType::Fl64 => write!(f, "FL64"),
            SemType::Fl128 => write!(f, "FL128"),
            SemType::Bool => write!(f, "BOOL"),
            SemType::Char => write!(f, "CHAR"),
            SemType::String => write!(f, "STRING"),
            SemType::Any => write!(f, "ANY"),
            SemType::Void => write!(f, "VOID"),
            SemType::List(elem) => write!(f, "LIST<{}>", elem),
            SemType::Set(elem) => write!(f, "SET<{}>", elem),
            SemType::Map(k, v) => write!(f, "MAP<{}, {}>", k, v),
            SemType::Union(members) => {
                for (i, m) in members.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", m)?;
                }
                Ok(())
            }
            SemType::Custom(name) => write!(f, "{}", name),
            SemType::Reference { inner, is_mut } => {
                if *is_mut {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            SemType::Unknown => write!(f, "<unknown>"),
        }
    }
}
