use std::collections::HashMap;

use super::manifest::CommandDecl;

/// Which kind of host runs a command. Only the built-in native host exists in
/// M1; a script host and a sandboxed host come later without the registry
/// changing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Host {
    Builtin,
}

#[derive(Clone, Debug)]
pub struct RegisteredCommand {
    pub extension_id: String,
    pub decl: CommandDecl,
    pub host: Host,
}

impl RegisteredCommand {
    pub fn qualified_id(&self) -> String {
        self.decl.qualified_id(&self.extension_id)
    }
}

#[derive(Default)]
pub struct Registry {
    commands: HashMap<String, RegisteredCommand>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Collision {
    pub qualified_id: String,
    pub existing_extension: String,
    pub rejected_extension: String,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an extension's commands. A qualified id already present belongs to
    /// another extension, so the incoming one is reported and skipped rather
    /// than overwriting a live command. Returns the collisions it refused.
    pub fn register(
        &mut self,
        extension_id: &str,
        commands: &[CommandDecl],
        host: Host,
    ) -> Vec<Collision> {
        let mut collisions = Vec::new();
        for decl in commands {
            let qualified = decl.qualified_id(extension_id);
            if let Some(existing) = self.commands.get(&qualified) {
                collisions.push(Collision {
                    qualified_id: qualified,
                    existing_extension: existing.extension_id.clone(),
                    rejected_extension: extension_id.to_string(),
                });
                continue;
            }
            self.commands.insert(
                qualified,
                RegisteredCommand {
                    extension_id: extension_id.to_string(),
                    decl: decl.clone(),
                    host,
                },
            );
        }
        collisions
    }

    pub fn unregister(&mut self, extension_id: &str) {
        self.commands.retain(|_, c| c.extension_id != extension_id);
    }

    pub fn get(&self, qualified_id: &str) -> Option<&RegisteredCommand> {
        self.commands.get(qualified_id)
    }

    pub fn commands(&self) -> impl Iterator<Item = &RegisteredCommand> {
        self.commands.values()
    }

    pub fn len(&self) -> usize {
        self.commands.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::manifest::InvocationMode;

    fn command(id: &str) -> CommandDecl {
        CommandDecl {
            id: id.into(),
            title: id.into(),
            mode: InvocationMode::View,
            subtitle: None,
            icon: None,
            keywords: vec![],
            alias: None,
        }
    }

    #[test]
    fn registers_and_resolves_by_qualified_id() {
        let mut registry = Registry::new();
        assert!(registry
            .register("apps", &[command("open")], Host::Builtin)
            .is_empty());
        let found = registry.get("apps.open").expect("registered");
        assert_eq!(found.extension_id, "apps");
        assert_eq!(found.host, Host::Builtin);
    }

    #[test]
    fn different_extensions_do_not_collide_on_a_shared_command_id() {
        let mut registry = Registry::new();
        assert!(registry
            .register("apps", &[command("open")], Host::Builtin)
            .is_empty());
        assert!(registry
            .register("other", &[command("open")], Host::Builtin)
            .is_empty());
        assert!(registry.get("apps.open").is_some());
        assert!(registry.get("other.open").is_some());
    }

    #[test]
    fn a_repeated_extension_id_collides_and_keeps_the_first() {
        let mut registry = Registry::new();
        registry.register("apps", &[command("open")], Host::Builtin);
        let collisions = registry.register("apps", &[command("open")], Host::Builtin);
        assert_eq!(
            collisions,
            vec![Collision {
                qualified_id: "apps.open".into(),
                existing_extension: "apps".into(),
                rejected_extension: "apps".into(),
            }]
        );
    }

    #[test]
    fn unregister_removes_only_that_extension() {
        let mut registry = Registry::new();
        registry.register("apps", &[command("open")], Host::Builtin);
        registry.register("calc", &[command("eval")], Host::Builtin);
        registry.unregister("apps");
        assert!(registry.get("apps.open").is_none());
        assert!(registry.get("calc.eval").is_some());
    }
}
