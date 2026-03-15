use crate::FxHashMap;

/// An interned symbol identifier — a cheap, copyable handle to a string.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymId(u32);

impl std::fmt::Debug for SymId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SymId({})", self.0)
    }
}

/// A string interner: deduplicates strings and hands out cheap `SymId` handles.
#[derive(Debug)]
pub struct Interner {
    map: FxHashMap<String, SymId>,
    vec: Vec<String>,
    /// Reusable scratch buffer for `intern_join` to avoid per-call allocation.
    join_buf: String,
}

impl Interner {
    pub fn new() -> Self {
        Self {
            map: FxHashMap::default(),
            vec: Vec::new(),
            join_buf: String::new(),
        }
    }

    /// Intern a string, returning its unique `SymId`. Creates a new entry if
    /// the string has not been seen before.
    pub fn intern(&mut self, s: &str) -> SymId {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = SymId(self.vec.len() as u32);
        let owned = s.to_owned();
        self.map.insert(owned.clone(), id);
        self.vec.push(owned);
        id
    }

    /// Intern the concatenation `"{a}.{b}"` without allocating a temporary
    /// String on every call. Uses an internal scratch buffer.
    pub fn intern_join(&mut self, a: SymId, b: SymId) -> SymId {
        self.join_buf.clear();
        self.join_buf.push_str(&self.vec[a.0 as usize]);
        self.join_buf.push('.');
        self.join_buf.push_str(&self.vec[b.0 as usize]);
        // Inline the intern logic to avoid borrow conflict with self.join_buf.
        if let Some(&id) = self.map.get(self.join_buf.as_str()) {
            return id;
        }
        let id = SymId(self.vec.len() as u32);
        let owned = self.join_buf.clone();
        self.map.insert(owned.clone(), id);
        self.vec.push(owned);
        id
    }

    /// Look up a string without creating a new entry. Returns `None` if the
    /// string has never been interned.
    pub fn lookup(&self, s: &str) -> Option<SymId> {
        self.map.get(s).copied()
    }

    /// Resolve a `SymId` back to its string.
    pub fn resolve(&self, id: SymId) -> &str {
        &self.vec[id.0 as usize]
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}
