use super::*;

impl AnalysisSession {
    /// Gather scope information by walking the cached AST.
    pub(super) fn build_scopes(module: &ModModule, module_ns: &str) -> FxHashMap<String, ScopeInfo> {
        let mut scopes: FxHashMap<String, ScopeInfo> = FxHashMap::default();

        let mut module_scope = ScopeInfo::new("");
        Self::collect_scope_defs(&module.body, &mut module_scope);
        scopes.insert(module_ns.to_string(), module_scope);
        Self::collect_nested_scopes(&module.body, module_ns, &mut scopes);

        scopes
    }

    pub(super) fn merge_scopes(&mut self, scopes: &FxHashMap<String, ScopeInfo>) {
        for (ns, sc) in scopes {
            if let Some(existing) = self.scopes.get_mut(ns.as_str()) {
                for (name, vs) in &sc.defs {
                    existing
                        .defs
                        .entry(name.clone())
                        .or_default()
                        .union_with(vs);
                }
                for (name, facts) in &sc.containers {
                    existing
                        .containers
                        .entry(name.clone())
                        .or_default()
                        .union_with(facts);
                }
                if existing.all_exports.is_none() && sc.all_exports.is_some() {
                    existing.all_exports = sc.all_exports.clone();
                }
            } else {
                self.scopes.insert(ns.clone(), sc.clone());
            }
        }
    }

    fn collect_scope_defs(stmts: &[Stmt], scope: &mut ScopeInfo) {
        for stmt in stmts {
            match stmt {
                Stmt::FunctionDef(f) => {
                    let name = f.name.id.to_string();
                    scope.defs.entry(name.clone()).or_default();
                    scope.locals.insert(name);
                }
                Stmt::ClassDef(c) => {
                    let name = c.name.id.to_string();
                    scope.defs.entry(name.clone()).or_default();
                    scope.locals.insert(name);
                }
                Stmt::Import(imp) => {
                    for alias in &imp.names {
                        let name = if let Some(ref asname) = alias.asname {
                            asname.id.to_string()
                        } else {
                            alias.name.id.to_string()
                        };
                        scope.defs.entry(name).or_default();
                    }
                }
                Stmt::ImportFrom(imp) => {
                    for alias in &imp.names {
                        if alias.name.id.as_str() == "*" {
                            continue;
                        }
                        let name = if let Some(ref asname) = alias.asname {
                            asname.id.to_string()
                        } else {
                            alias.name.id.to_string()
                        };
                        scope.defs.entry(name).or_default();
                    }
                }
                Stmt::Assign(a) => {
                    for target in &a.targets {
                        if let Expr::Name(n) = target
                            && n.id.as_str() == "__all__"
                            && let Some(exports) = extract_all_exports(&a.value)
                        {
                            scope.all_exports = Some(exports);
                        }
                        Self::collect_assign_target_names(target, scope);
                    }
                }
                Stmt::AugAssign(a) => Self::collect_assign_target_names(&a.target, scope),
                Stmt::AnnAssign(a) => Self::collect_assign_target_names(&a.target, scope),
                Stmt::For(f) => {
                    Self::collect_assign_target_names(&f.target, scope);
                    Self::collect_scope_defs(&f.body, scope);
                    Self::collect_scope_defs(&f.orelse, scope);
                }
                Stmt::With(w) => {
                    for item in &w.items {
                        if let Some(vars) = &item.optional_vars {
                            Self::collect_assign_target_names(vars, scope);
                        }
                    }
                    Self::collect_scope_defs(&w.body, scope);
                }
                Stmt::Try(s) => {
                    for handler in &s.handlers {
                        let ExceptHandler::ExceptHandler(h) = handler;
                        if let Some(name) = &h.name {
                            let name = name.id.to_string();
                            scope.defs.entry(name.clone()).or_default();
                            scope.locals.insert(name);
                        }
                    }
                    Self::collect_scope_defs(&s.body, scope);
                    for handler in &s.handlers {
                        let ExceptHandler::ExceptHandler(h) = handler;
                        Self::collect_scope_defs(&h.body, scope);
                    }
                    Self::collect_scope_defs(&s.orelse, scope);
                    Self::collect_scope_defs(&s.finalbody, scope);
                }
                Stmt::If(s) => {
                    Self::collect_scope_defs(&s.body, scope);
                    for clause in &s.elif_else_clauses {
                        Self::collect_scope_defs(&clause.body, scope);
                    }
                }
                Stmt::While(s) => {
                    Self::collect_scope_defs(&s.body, scope);
                    Self::collect_scope_defs(&s.orelse, scope);
                }
                _ => {}
            }
        }
    }

    fn collect_nested_scopes(
        stmts: &[Stmt],
        parent_ns: &str,
        scopes: &mut FxHashMap<String, ScopeInfo>,
    ) {
        for stmt in stmts {
            match stmt {
                Stmt::FunctionDef(f) => {
                    let name = f.name.id.to_string();
                    let ns = format!("{parent_ns}.{name}");
                    let mut scope = ScopeInfo::new(&name);
                    for a in &f.parameters.posonlyargs {
                        let pname = a.parameter.name.id.to_string();
                        scope.defs.entry(pname.clone()).or_default();
                        scope.locals.insert(pname);
                    }
                    for a in &f.parameters.args {
                        let pname = a.parameter.name.id.to_string();
                        scope.defs.entry(pname.clone()).or_default();
                        scope.locals.insert(pname);
                    }
                    for a in &f.parameters.kwonlyargs {
                        let pname = a.parameter.name.id.to_string();
                        scope.defs.entry(pname.clone()).or_default();
                        scope.locals.insert(pname);
                    }
                    if let Some(ref va) = f.parameters.vararg {
                        let pname = va.name.id.to_string();
                        scope.defs.entry(pname.clone()).or_default();
                        scope.locals.insert(pname);
                    }
                    if let Some(ref kw) = f.parameters.kwarg {
                        let pname = kw.name.id.to_string();
                        scope.defs.entry(pname.clone()).or_default();
                        scope.locals.insert(pname);
                    }

                    Self::collect_scope_defs(&f.body, &mut scope);
                    scopes.insert(ns.clone(), scope);
                    Self::collect_nested_scopes(&f.body, &ns, scopes);
                }
                Stmt::ClassDef(c) => {
                    let name = c.name.id.to_string();
                    let ns = format!("{parent_ns}.{name}");
                    let mut scope = ScopeInfo::new(&name);
                    Self::collect_scope_defs(&c.body, &mut scope);
                    scopes.insert(ns.clone(), scope);
                    Self::collect_nested_scopes(&c.body, &ns, scopes);
                }
                Stmt::If(s) => {
                    Self::collect_nested_scopes(&s.body, parent_ns, scopes);
                    for clause in &s.elif_else_clauses {
                        Self::collect_nested_scopes(&clause.body, parent_ns, scopes);
                    }
                }
                Stmt::While(s) => {
                    Self::collect_nested_scopes(&s.body, parent_ns, scopes);
                    Self::collect_nested_scopes(&s.orelse, parent_ns, scopes);
                }
                Stmt::For(s) => {
                    Self::collect_nested_scopes(&s.body, parent_ns, scopes);
                    Self::collect_nested_scopes(&s.orelse, parent_ns, scopes);
                }
                Stmt::With(s) => Self::collect_nested_scopes(&s.body, parent_ns, scopes),
                Stmt::Try(s) => {
                    Self::collect_nested_scopes(&s.body, parent_ns, scopes);
                    for handler in &s.handlers {
                        let ExceptHandler::ExceptHandler(h) = handler;
                        Self::collect_nested_scopes(&h.body, parent_ns, scopes);
                    }
                    Self::collect_nested_scopes(&s.orelse, parent_ns, scopes);
                    Self::collect_nested_scopes(&s.finalbody, parent_ns, scopes);
                }
                _ => {}
            }
        }
    }

    fn collect_assign_target_names(target: &Expr, scope: &mut ScopeInfo) {
        match target {
            Expr::Name(n) => {
                let name = n.id.to_string();
                scope.defs.entry(name.clone()).or_default();
                scope.locals.insert(name);
            }
            Expr::Tuple(t) => {
                for elt in &t.elts {
                    Self::collect_assign_target_names(elt, scope);
                }
            }
            Expr::List(l) => {
                for elt in &l.elts {
                    Self::collect_assign_target_names(elt, scope);
                }
            }
            Expr::Starred(s) => Self::collect_assign_target_names(&s.value, scope),
            _ => {}
        }
    }
}
