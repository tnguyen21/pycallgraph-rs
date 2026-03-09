use std::collections::{HashMap, HashSet};

/// Tracks name bindings within a lexical scope.
#[derive(Debug, Clone)]
pub struct Scope {
    /// The fully qualified name of this scope (e.g., "module.Class.method").
    pub name: String,
    /// Names defined (bound) in this scope.
    pub defs: HashMap<String, Option<String>>,
    /// Names that are local-only (assigned in this scope).
    pub locals: HashSet<String>,
}

impl Scope {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            defs: HashMap::new(),
            locals: HashSet::new(),
        }
    }

    /// Bind a name in this scope, optionally to a fully qualified target.
    pub fn bind(&mut self, name: &str, target: Option<&str>) {
        self.defs
            .insert(name.to_string(), target.map(|s| s.to_string()));
        self.locals.insert(name.to_string());
    }

    /// Look up a name in this scope.
    pub fn get(&self, name: &str) -> Option<&Option<String>> {
        self.defs.get(name)
    }

    /// Check if a name is defined in this scope.
    pub fn has(&self, name: &str) -> bool {
        self.defs.contains_key(name)
    }
}

/// A stack of scopes for lexical name resolution.
#[derive(Debug)]
pub struct ScopeStack {
    scopes: Vec<Scope>,
}

impl Default for ScopeStack {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeStack {
    pub fn new() -> Self {
        Self { scopes: Vec::new() }
    }

    pub fn push(&mut self, scope: Scope) {
        self.scopes.push(scope);
    }

    pub fn pop(&mut self) -> Option<Scope> {
        self.scopes.pop()
    }

    pub fn current(&self) -> Option<&Scope> {
        self.scopes.last()
    }

    pub fn current_mut(&mut self) -> Option<&mut Scope> {
        self.scopes.last_mut()
    }

    /// Look up a name by walking scopes from innermost to outermost.
    /// Returns the fully qualified target if bound, or None.
    pub fn resolve(&self, name: &str) -> Option<String> {
        for scope in self.scopes.iter().rev() {
            if let Some(target) = scope.defs.get(name) {
                return target.clone();
            }
        }
        None
    }

    /// Check if a name is defined in any scope.
    pub fn is_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|s| s.defs.contains_key(name))
    }

    /// Get the current namespace (fully qualified scope name).
    pub fn current_namespace(&self) -> String {
        self.scopes
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(".")
    }

    pub fn depth(&self) -> usize {
        self.scopes.len()
    }
}
