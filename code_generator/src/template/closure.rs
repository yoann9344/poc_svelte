use quote::quote;
pub use sailfish::TemplateOnce;
use syn::{visit_mut::VisitMut, Block, Expr, ExprBlock, Ident};

use super::component::clean_up_generated;
use crate::{
    state_block::{EventClosure, State},
    visitor::IdentModifier,
};

#[derive(TemplateOnce)]
#[template(path = "bind_input_closure.rs", escape = false)]
pub struct ClosureBindInputTemplate {
    name: String,
    bind_ident: Ident,
    number_type: Option<String>,
}

pub struct ClosureBindInput {
    pub callback_on_change: (String, String),
    pub init_value: String,
    pub update_value: String,
}

impl ClosureBindInput {
    pub fn new(element_name: &str, init_ident: String, state: &State) -> ClosureBindInput {
        println!("Closure bind template");
        let name = &format!("{element_name}_bind_value");
        let init_value;
        let update_value;

        let mut number_type = None;
        let State { ident, ty, .. } = state;
        let ty = quote!(#ty).to_string();
        let is_number = vec![
            "u8", "u16", "u32", "u64", "i8", "i16", "i32", "i64", "f32", "f64",
        ]
        .iter()
        .cloned()
        .collect::<String>()
        .contains(&ty);

        let value;
        if is_number {
            number_type = Some(ty.clone());
            value = format!("state.borrow().{} as f64", init_ident);
        } else {
            value = format!("state.borrow().{}", init_ident);
        }
        init_value = format!("{}.set_value_as_number({});", element_name, value);
        update_value = format!("self.{}.set_value_as_number(self.{});", element_name, value);

        let template = ClosureBindInputTemplate {
            name: name.to_string(),
            bind_ident: ident.clone(),
            number_type,
        };
        Self {
            callback_on_change: (
                name.to_string(),
                clean_up_generated(template.render_once().unwrap()),
            ),
            init_value,
            update_value,
        }
    }
}

#[derive(TemplateOnce)]
#[template(path = "closure.rs", escape = false)]
pub struct ClosureTemplate {
    name: String,
    statements: Vec<String>,
}

impl ClosureTemplate {
    pub fn string_from_event_closure(
        event_closure: &mut EventClosure,
        ident_modifier: &mut IdentModifier,
    ) -> (String, String) {
        ident_modifier.visit_expr_closure_mut(&mut event_closure.closure);
        let name = event_closure.ident.to_string();
        let statements;
        match *event_closure.closure.body.to_owned() {
            Expr::Block(ExprBlock {
                block: Block { stmts, .. },
                ..
            }) => statements = stmts.iter().map(|s| quote!(#s).to_string()).collect(),
            expr => statements = vec![quote!(#expr;).to_string()],
        }
        (
            name.clone(),
            clean_up_generated(Self { name, statements }.render_once().unwrap()),
        )
    }
}
