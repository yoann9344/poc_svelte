use syn::{Block, Error, Expr, ExprClosure, Ident, Local, Pat, PatIdent, PatType, Result, Type};

#[derive(Debug)]
pub struct State {
    pub ident: Ident,
    pub ty: Type,
    pub local: Local,
}

#[derive(Debug)]
pub struct EventClosure {
    pub ident: Ident,
    pub closure: ExprClosure,
    pub local: Local,
}

#[derive(Debug, Default)]
pub struct LocalDetails {
    pub states: Vec<State>,
    pub events_closures: Vec<EventClosure>,
    pub block: String,
}

impl LocalDetails {
    pub fn get_ident_modifier(&self, state_ident: &str) -> super::visitor::IdentModifier {
        println!(
            "LocalDetails.states.idents : {:?}",
            self.states
                .iter()
                .map(|State { ident, .. }| ident.to_string())
                .collect::<Vec<_>>()
        );
        super::visitor::IdentModifier::new(
            self.states
                .iter()
                .map(|State { ident, .. }| ident.to_string())
                .collect(),
            state_ident.to_string(),
        )
    }

    pub fn states_contains_ident(&self, ident: &Ident) -> Result<()> {
        if self
            .states
            .iter()
            .any(|state| state.ident.to_string() == ident.to_string())
        {
            Ok(())
        } else {
            Err(Error::new(
                ident.span(),
                "Variable is not declared in state's block.",
            ))
        }
    }
    pub fn events_contains_ident(&self, ident: &Ident) -> Result<()> {
        if self
            .events_closures
            .iter()
            .any(|event_closure| event_closure.ident.to_string() == ident.to_string())
        {
            Ok(())
        } else {
            Err(Error::new(
                ident.span(),
                "event's callback not declared in state's block.",
            ))
        }
    }
}

// fn extract_locals(block: &Block) -> Vec<LocalDetails> {
pub fn extract_locals(block: &Block) -> Result<LocalDetails> {
    let mut details = LocalDetails::default();
    details.block = block
        .stmts
        .iter()
        .map(|stmt| quote::quote!(#stmt).to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let locals: Vec<Local> = block
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Local(local) => Some(local),
            _ => None,
        })
        .cloned()
        .collect();
    for local in locals {
        match local {
            Local {
                pat:
                    Pat::Type(PatType {
                        pat: ref pat_ident,
                        ty: ref path_type,
                        ..
                    }),
                ..
            } => {
                // assert_eq!(quote::quote!(#path_type).to_string(), "u32");
                match *pat_ident.clone() {
                    Pat::Ident(PatIdent { ident, .. }) => {
                        details.states.push(State {
                            ty: *path_type.clone(),
                            ident: ident.clone(),
                            local: local.clone(),
                        });
                    }
                    _ => panic!("FIXME: get ident from PatIdent"),
                };
            }
            Local {
                pat: Pat::Ident(PatIdent { ref ident, .. }),
                ref init,
                ..
            } => {
                if let Some((_, boxed_expr)) = init {
                    if let Expr::Closure(ref closure) = **boxed_expr {
                        details.events_closures.push(EventClosure {
                            ident: ident.clone(),
                            closure: closure.clone(),
                            local: local.clone(),
                        });
                        continue;
                    }
                }
                Err(Error::new(
                    ident.span(),
                    "You must add a type. (Automatic type detection is not yet implemented)",
                ))?;
            }
            _ => Err(Error::new(
                local.let_token.span,
                "This type of local is not handled yet.",
            ))?,
        }
    }
    Ok(details)
}
