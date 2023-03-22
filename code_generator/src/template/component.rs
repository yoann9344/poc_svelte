use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
pub use sailfish::TemplateOnce;
use syn::{
    parse_str, punctuated::Punctuated, token::Comma, Field, FieldValue, Ident, Member, Visibility,
};

use crate::{
    html::Element,
    state_block::{LocalDetails, State},
    visitor::IdentModifier,
};

use super::{nodes::Dom, ClosureTemplate};

#[derive(TemplateOnce)]
#[template(path = "component.rs", escape = false)]
pub struct Component {
    pub state_init_block: String,
    pub state_fields_type: Vec<String>,
    pub state_fields_value: Vec<String>,
    pub callbacks: Vec<(String, String)>,
    pub init: Vec<String>,
    pub update: Vec<String>,
    pub drop: Vec<String>,
    pub dom_state: Vec<(String, String)>,
}

pub fn clean_up_generated(generated: String) -> String {
    generated.as_str().replace("//ITEM ", "")
}

impl Component {
    pub fn new(local_details: &mut LocalDetails, elements: &Vec<Element>) -> Self {
        let state_init_block = local_details
            .states
            .iter()
            .map(|State { local, .. }| quote! {#local}.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        let state_fields_type = local_details
            .states
            .iter()
            .map(|details| {
                gen_field_type(
                    &details.ident.to_string(),
                    details.ty.to_token_stream().to_string().as_str(),
                )
            })
            .collect();
        let state_fields_value = local_details
            .states
            .iter()
            .map(|details| gen_field_value_shorthand(&details.ident.to_string()))
            .collect();

        let mut ident_modifier = IdentModifier::new(
            local_details
                .states
                .iter()
                .map(|State { ident, .. }| ident.to_string())
                .collect(),
        );

        let mut callbacks: Vec<(String, String)> = local_details
            .events_closures
            .iter_mut()
            .map(|event_closure| {
                ClosureTemplate::string_from_event_closure(event_closure, &mut ident_modifier)
            })
            .collect();

        // // TODO: Handle binding dynamically
        // callbacks.push(ClosureBindInputTemplate::string(
        //     "input_0_bind_value",
        //     local_details.states.get(0).unwrap(),
        // ));
        let mut dom = Dom::generate(elements, local_details);
        callbacks.append(&mut dom.binded_callbacks);

        let Dom {
            init,
            update,
            drop,
            dom_state,
            ..
        } = dom;

        Self {
            state_init_block,
            state_fields_type,
            state_fields_value,
            callbacks,
            init,
            update,
            drop,
            dom_state, // callbacks: vec!["".to_string(), "".to_string()],
        }
    }
    pub fn to_token_stream(self) -> TokenStream {
        let generated: String = clean_up_generated(self.render_once().unwrap());
        generated.parse().unwrap()
    }
}

pub fn gen_field_type(ident: &str, ty: &str) -> String {
    println!("ident {} and ty {}", ident, ty);
    let mut punctuated_fields: Punctuated<Field, Comma> = Punctuated::new();
    let field = Field {
        attrs: vec![],
        vis: Visibility::Inherited,
        ident: Some(Ident::new(ident, Span::call_site())),
        colon_token: Some(parse_str(":").unwrap()),
        ty: parse_str(ty).unwrap(),
    };
    punctuated_fields.push_value(field);
    punctuated_fields.push_punct(parse_str(",").unwrap());
    quote!(#punctuated_fields).to_string()
}

pub fn gen_field_value_shorthand(ident: &str) -> String {
    let mut punctuated_fields: Punctuated<FieldValue, Comma> = Punctuated::new();
    let field = FieldValue {
        attrs: vec![],
        member: Member::Named(Ident::new(ident, Span::call_site())),
        colon_token: None,
        expr: parse_str(ident).unwrap(),
    };
    // println!("{:?}", field.expr);
    punctuated_fields.push_value(field);
    punctuated_fields.push_punct(parse_str(",").unwrap());
    quote!(#punctuated_fields).to_string()
}
