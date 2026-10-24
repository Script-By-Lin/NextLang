use crate::types::SemType;
use nx_ast::AccessModifier;
use nx_diagnostics::{DiagnosticBag, Span};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ClassFieldSymbol {
    pub name: String,
    pub ty: SemType,
    pub access: AccessModifier,
    pub is_static: bool,
    pub is_const: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassMethodSymbol {
    pub name: String,
    pub param_names: Vec<String>,
    pub param_types: Vec<SemType>,
    pub return_type: SemType,
    pub access: AccessModifier,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ClassSymbol {
    pub name: String,
    pub is_abstract: bool,
    pub is_final: bool,
    pub parent: Option<String>,
    pub parent_span: Option<Span>,
    pub implements: Vec<(String, Span)>,
    pub fields: HashMap<String, ClassFieldSymbol>,
    pub methods: HashMap<String, ClassMethodSymbol>,
    pub span: Span,
}

impl ClassSymbol {
    pub fn constructor(&self) -> Option<&ClassMethodSymbol> {
        self.methods.get("init")
    }
}

#[derive(Debug, Clone)]
pub struct InterfaceMethodSymbol {
    pub name: String,
    pub param_names: Vec<String>,
    pub param_types: Vec<SemType>,
    pub return_type: SemType,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterfaceSymbol {
    pub name: String,
    pub methods: HashMap<String, InterfaceMethodSymbol>,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct ClassHierarchy {
    pub classes: HashMap<String, ClassSymbol>,
    pub interfaces: HashMap<String, InterfaceSymbol>,
}

impl ClassHierarchy {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn lookup_class(&self, name: &str) -> Option<&ClassSymbol> {
        self.classes.get(name)
    }

    pub fn lookup_interface(&self, name: &str) -> Option<&InterfaceSymbol> {
        self.interfaces.get(name)
    }

    pub fn is_type_declared(&self, name: &str) -> bool {
        self.classes.contains_key(name) || self.interfaces.contains_key(name)
    }

    /// Checks if `sub` is a subtype of `sup` (subclass or interface implementor).
    pub fn is_subtype(&self, sub: &str, sup: &str) -> bool {
        if sub == sup {
            return true;
        }

        let mut current = sub;
        let mut visited = HashSet::new();

        while let Some(cls) = self.classes.get(current) {
            if !visited.insert(current.to_string()) {
                break; // Cycle protection
            }

            // Check if class directly implements `sup`
            for (iface_name, _) in &cls.implements {
                if iface_name == sup {
                    return true;
                }
            }

            // Check parent class
            if let Some(ref parent) = cls.parent {
                if parent == sup {
                    return true;
                }
                current = parent;
            } else {
                break;
            }
        }

        false
    }

    /// Walks up the class inheritance chain to find a field.
    pub fn lookup_field(
        &self,
        class_name: &str,
        field_name: &str,
    ) -> Option<(&ClassSymbol, &ClassFieldSymbol)> {
        let mut current = class_name;
        let mut visited = HashSet::new();

        while let Some(cls) = self.classes.get(current) {
            if !visited.insert(current.to_string()) {
                break;
            }
            if let Some(field) = cls.fields.get(field_name) {
                return Some((cls, field));
            }
            if let Some(ref parent) = cls.parent {
                current = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Walks up the class inheritance chain to find a method.
    pub fn lookup_method(
        &self,
        class_name: &str,
        method_name: &str,
    ) -> Option<(&ClassSymbol, &ClassMethodSymbol)> {
        let mut current = class_name;
        let mut visited = HashSet::new();

        while let Some(cls) = self.classes.get(current) {
            if !visited.insert(current.to_string()) {
                break;
            }
            if let Some(method) = cls.methods.get(method_name) {
                return Some((cls, method));
            }
            if let Some(ref parent) = cls.parent {
                current = parent;
            } else {
                break;
            }
        }
        None
    }

    /// Checks visibility: can the current context access a member declared in `declaring_class`?
    pub fn can_access(
        &self,
        declaring_class: &str,
        access: AccessModifier,
        current_class: Option<&str>,
    ) -> bool {
        match access {
            AccessModifier::Public => true,
            AccessModifier::Private => current_class == Some(declaring_class),
            AccessModifier::Protected => {
                if let Some(curr) = current_class {
                    curr == declaring_class || self.is_subtype(curr, declaring_class)
                } else {
                    false
                }
            }
        }
    }

    /// Validates complete class hierarchy: inheritance cycles, unknown parents,
    /// abstract method fulfillment, interface fulfillment, override compatibility, and final constraints.
    pub fn validate_hierarchy(&self, diagnostics: &mut DiagnosticBag) {
        // 1. Check parent classes and cycles
        for (name, cls) in &self.classes {
            if let Some(ref parent_name) = cls.parent {
                let parent_span = cls.parent_span.unwrap_or(cls.span);

                // Parent must exist
                if let Some(parent_cls) = self.classes.get(parent_name) {
                    // Parent cannot be final
                    if parent_cls.is_final {
                        diagnostics.error_code_with_help(
                            "NX-OOP-007",
                            parent_span,
                            format!("cannot extend final class '{}'", parent_name),
                            "remove 'final' modifier from parent class or do not extend it",
                        );
                    }
                } else if self.interfaces.contains_key(parent_name) {
                    diagnostics.error_code_with_help(
                        "NX-OOP-001",
                        parent_span,
                        format!("cannot extend interface '{}' using 'extends'", parent_name),
                        "use 'implements' to implement interfaces",
                    );
                } else {
                    diagnostics.error_code_with_help(
                        "NX-OOP-001",
                        parent_span,
                        format!(
                            "unknown parent class '{}' in declaration of '{}'",
                            parent_name, name
                        ),
                        "ensure parent class is declared in the file",
                    );
                }

                // Check inheritance cycles
                let mut path = vec![name.clone()];
                let mut current = parent_name.clone();
                while let Some(next_cls) = self.classes.get(&current) {
                    if path.contains(&current) {
                        path.push(current);
                        diagnostics.error_code_with_help(
                            "NX-OOP-002",
                            cls.span,
                            format!("circular inheritance detected: {}", path.join(" -> ")),
                            "remove circular dependency in class hierarchy",
                        );
                        break;
                    }
                    path.push(current.clone());
                    if let Some(ref next_parent) = next_cls.parent {
                        current = next_parent.clone();
                    } else {
                        break;
                    }
                }
            }

            // 2. Check implemented interfaces
            for (iface_name, iface_span) in &cls.implements {
                if !self.interfaces.contains_key(iface_name) {
                    if self.classes.contains_key(iface_name) {
                        diagnostics.error_code_with_help(
                            "NX-OOP-011",
                            *iface_span,
                            format!(
                                "'{}' is a class, not an interface; use 'extends' instead",
                                iface_name
                            ),
                            "use 'extends' for parent classes",
                        );
                    } else {
                        diagnostics.error_code_with_help(
                            "NX-OOP-011",
                            *iface_span,
                            format!(
                                "unknown interface '{}' implemented by '{}'",
                                iface_name, name
                            ),
                            "ensure interface is declared in the file",
                        );
                    }
                }
            }

            // 3. Check abstract methods in non-abstract class
            if !cls.is_abstract {
                for method in cls.methods.values() {
                    if method.is_abstract {
                        diagnostics.error_code_with_help(
                            "NX-OOP-017",
                            method.span,
                            format!("abstract method '{}' cannot be declared in non-abstract class '{}'", method.name, name),
                            "declare class as 'abstract class' or provide a method body",
                        );
                    }
                }
            }

            // 4. Check method overrides against parent classes
            if let Some(ref parent_name) = cls.parent {
                if let Some(parent_cls) = self.classes.get(parent_name) {
                    for (method_name, method) in &cls.methods {
                        // Constructors are class-specific and not virtual method overrides
                        if method_name == "init" {
                            continue;
                        }
                        if let Some((_ancestor, parent_method)) =
                            self.lookup_in_ancestors(parent_cls, method_name)
                        {
                            // Check if parent method is final
                            if parent_method.is_final {
                                diagnostics.error_code_with_help(
                                    "NX-OOP-008",
                                    method.span,
                                    format!("cannot override final method '{}' from parent class '{}'", method_name, parent_name),
                                    "remove 'final' modifier from parent method or rename this method",
                                );
                            }

                            // Check parameter count
                            if method.param_types.len() != parent_method.param_types.len() {
                                diagnostics.error_code_with_help(
                                    "NX-OOP-009",
                                    method.span,
                                    format!(
                                        "incompatible override of method '{}': expected {} parameters, found {}",
                                        method_name,
                                        parent_method.param_types.len(),
                                        method.param_types.len()
                                    ),
                                    "override parameter count must match parent method signature exactly",
                                );
                            } else {
                                // Check parameter types
                                for (i, (child_ty, par_ty)) in method
                                    .param_types
                                    .iter()
                                    .zip(&parent_method.param_types)
                                    .enumerate()
                                {
                                    if child_ty != par_ty {
                                        diagnostics.error_code_with_help(
                                            "NX-OOP-009",
                                            method.span,
                                            format!(
                                                "incompatible parameter {} type in override of '{}': expected '{}', found '{}'",
                                                i + 1,
                                                method_name,
                                                par_ty,
                                                child_ty
                                            ),
                                            "override parameter types must match parent method signature exactly",
                                        );
                                    }
                                }
                            }

                            // Check return type
                            if method.return_type != parent_method.return_type {
                                diagnostics.error_code_with_help(
                                    "NX-OOP-009",
                                    method.span,
                                    format!(
                                        "incompatible return type in override of '{}': expected '{}', found '{}'",
                                        method_name,
                                        parent_method.return_type,
                                        method.return_type
                                    ),
                                    "override return type must match parent method return type exactly",
                                );
                            }
                        }
                    }
                }
            }

            // 5. If concrete class, check that all inherited abstract methods are implemented
            if !cls.is_abstract {
                let mut current = cls.parent.as_ref();
                let mut visited = HashSet::new();

                while let Some(parent_name) = current {
                    if !visited.insert(parent_name.clone()) {
                        break;
                    }
                    if let Some(parent_cls) = self.classes.get(parent_name) {
                        for (method_name, abs_method) in &parent_cls.methods {
                            if abs_method.is_abstract {
                                // Must be implemented in cls or intermediate concrete class
                                let implemented = self
                                    .lookup_method(name, method_name)
                                    .is_some_and(|(_, m)| !m.is_abstract);

                                if !implemented {
                                    diagnostics.error_code_with_help(
                                        "NX-OOP-010",
                                        cls.span,
                                        format!(
                                            "class '{}' must implement inherited abstract method '{}' from '{}'",
                                            name, method_name, parent_name
                                        ),
                                        format!("implement 'public fn {}' in class '{}' or mark '{}' as abstract", method_name, name, name),
                                    );
                                }
                            }
                        }
                        current = parent_cls.parent.as_ref();
                    } else {
                        break;
                    }
                }
            }

            // 6. If concrete class, check that all interface methods are implemented
            if !cls.is_abstract {
                for (iface_name, _) in &cls.implements {
                    if let Some(iface) = self.interfaces.get(iface_name) {
                        for (method_name, iface_method) in &iface.methods {
                            match self.lookup_method(name, method_name) {
                                None => {
                                    diagnostics.error_code_with_help(
                                        "NX-OOP-011",
                                        cls.span,
                                        format!(
                                            "class '{}' fails to implement interface method '{}' required by '{}'",
                                            name, method_name, iface_name
                                        ),
                                        format!("implement 'public fn {}(...):<{}>' in class '{}'", method_name, iface_method.return_type, name),
                                    );
                                }
                                Some((_, method)) => {
                                    // Must be public
                                    if method.access != AccessModifier::Public {
                                        diagnostics.error_code_with_help(
                                            "NX-OOP-011",
                                            method.span,
                                            format!(
                                                "interface method '{}' implemented by '{}' must have public visibility",
                                                method_name, name
                                            ),
                                            "change access modifier to 'public'",
                                        );
                                    }
                                    // Check signature matching
                                    if method.param_types.len() != iface_method.param_types.len() {
                                        diagnostics.error_code_with_help(
                                            "NX-OOP-011",
                                            method.span,
                                            format!(
                                                "method '{}' in '{}' has {} parameters, but interface '{}' requires {}",
                                                method_name, name, method.param_types.len(), iface_name, iface_method.param_types.len()
                                            ),
                                            "method parameter count must match interface declaration",
                                        );
                                    } else {
                                        for (i, (m_ty, if_ty)) in method
                                            .param_types
                                            .iter()
                                            .zip(&iface_method.param_types)
                                            .enumerate()
                                        {
                                            if m_ty != if_ty {
                                                diagnostics.error_code_with_help(
                                                    "NX-OOP-011",
                                                    method.span,
                                                    format!(
                                                        "method '{}' in '{}' parameter {} type '{}' does not match interface type '{}'",
                                                        method_name, name, i + 1, m_ty, if_ty
                                                    ),
                                                    "parameter types must match interface declaration exactly",
                                                );
                                            }
                                        }
                                    }
                                    if method.return_type != iface_method.return_type {
                                        diagnostics.error_code_with_help(
                                            "NX-OOP-011",
                                            method.span,
                                            format!(
                                                "method '{}' in '{}' return type '{}' does not match interface return type '{}'",
                                                method_name, name, method.return_type, iface_method.return_type
                                            ),
                                            "return type must match interface declaration exactly",
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn lookup_in_ancestors<'a>(
        &'a self,
        start_class: &'a ClassSymbol,
        method_name: &str,
    ) -> Option<(&'a ClassSymbol, &'a ClassMethodSymbol)> {
        let mut current = Some(start_class);
        let mut visited = HashSet::new();

        while let Some(cls) = current {
            if !visited.insert(cls.name.clone()) {
                break;
            }
            if let Some(m) = cls.methods.get(method_name) {
                return Some((cls, m));
            }
            current = cls.parent.as_ref().and_then(|p| self.classes.get(p));
        }
        None
    }
}
