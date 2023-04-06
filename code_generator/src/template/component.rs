use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
};

use super::{
    nodes::{Dom, State as StateTemplate},
    ClosureTemplate,
};

#[derive(TemplateOnce)]
#[template(path = "component.rs", escape = false)]
pub struct Component {
    // pub init_block: String,
    // pub fields_type: Vec<String>,
    // pub fields_value: Vec<String>,
    // pub callbacks: Vec<(String, String)>,
    // pub init: Vec<String>,
    // pub mount: Vec<String>,
    // pub mount_mounted: Vec<String>,
    // pub update: Vec<String>,
    // pub drop: Vec<String>,
    // pub dom_state: Vec<(String, String)>,
    pub dom: Dom,
}

pub fn clean_up_generated(generated: String) -> String {
    generated.as_str().replace("//ITEM ", "")
}

impl Component {
    pub fn new(local_details: &mut LocalDetails, elements: &Vec<Element>) -> Self {
        let init_block = local_details
            .states
            .iter()
            .map(|State { local, .. }| quote! {#local}.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        let fields_type = local_details
            .states
            .iter()
            .map(|details| {
                gen_field_type(
                    &details.ident.to_string(),
                    details.ty.to_token_stream().to_string().as_str(),
                )
            })
            .collect();
        let fields_value = local_details
            .states
            .iter()
            .map(|details| gen_field_value_shorthand(&details.ident.to_string()))
            .collect();

        // let mut ident_modifier = IdentModifier::new(
        //     local_details
        //         .states
        //         .iter()
        //         .map(|State { ident, .. }| ident.to_string())
        //         .collect(),
        // );
        let mut ident_modifier = local_details.get_ident_modifier("s");
        println!(
            "Event closure names : {:?}",
            local_details
                .events_closures
                .iter()
                .map(|ec| ec.ident.to_string())
                .collect::<Vec<_>>()
        );

        let callbacks: Vec<(String, String)> = local_details
            .events_closures
            .iter_mut()
            .map(|event_closure| {
                ClosureTemplate::string_from_event_closure(event_closure, &mut ident_modifier)
            })
            .collect();

        let state = Rc::new(RefCell::new(StateTemplate {
            init_block,
            fields_type,
            fields_value,
            callbacks,
        }));
        let dom = Dom::generate(elements, local_details, HashMap::new(), state.clone(), true);

        Self { dom }
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
