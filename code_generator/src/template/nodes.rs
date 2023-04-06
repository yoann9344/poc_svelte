use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::quote;
use syn::{visit::Visit, visit_mut::VisitMut};
use syn::{ExprForLoop, Ident};

use crate::thir::get_pat_bindings;
use crate::{
    html::{AttrExprType, Attribute, Classic, Condition, Element, ExprElement},
    state_block::LocalDetails,
    visitor::ident::IdentExtractor,
};
pub use sailfish::TemplateOnce;

use super::ClosureBindInput;

macro_rules! format_ident {
    ($props:expr, $ident:ident: mut) => {{
        if $props.contains_key(&$ident.to_string()) {
            format!("props.{}", $ident)
        } else {
            format!("state.borrow_mut().{}", $ident)
        }
    }};
    ($props:expr, $ident:ident: self mut) => {{
        if $props.contains_key(&$ident.to_string()) {
            format!("self.props.{}", $ident)
        } else {
            format!("self.state.borrow_mut().{}", $ident)
        }
    }};
    ($props:expr, $ident:ident: self) => {{
        if $props.contains_key(&$ident.to_string()) {
            format!("self.props.{}", $ident)
        } else {
            format!("self.state.borrow().{}", $ident)
        }
    }};
    ($props:expr, $ident:ident) => {{
        if $props.contains_key(&$ident.to_string()) {
            format!("props.{}", $ident)
        } else {
            format!("state.borrow().{}", $ident)
        }
    }};
}

#[derive(Clone, TemplateOnce)]
#[template(path = "state.rs", escape = false)]
pub struct State {
    pub init_block: String,
    pub fields_type: Vec<String>,
    pub fields_value: Vec<String>,
    pub callbacks: Vec<(String, String)>,
}

#[derive(TemplateOnce)]
#[template(path = "dom.rs", escape = false)]
pub struct Dom {
    pub init: Vec<String>,
    pub mount: Vec<String>,
    pub mount_mounted: Vec<String>,
    pub update: Vec<String>,
    pub drop: Vec<String>,
    // pub binded_callbacks: Vec<(String, String)>,
    pub fields: HashMap<String, String>,
    pub sub_doms: HashMap<String, Dom>,
    pub props: HashMap<String, String>,
    // data not used in template (most start with _)
    pub _state: Rc<RefCell<State>>,
    _tag_count: HashMap<String, usize>,
    _append_nodes: bool,
}

impl Dom {
    fn default(_state: Rc<RefCell<State>>, props: HashMap<String, String>) -> Self {
        Self {
            init: Vec::new(),
            mount: Vec::new(),
            mount_mounted: Vec::new(),
            update: Vec::new(),
            drop: Vec::new(),
            fields: HashMap::new(),
            props,
            sub_doms: HashMap::new(),
            _state,
            _tag_count: HashMap::new(),
            _append_nodes: true,
        }
    }
}

impl Dom {
    pub fn generate(
        elements: &Vec<Element>,
        details: &LocalDetails,
        props: HashMap<String, String>,
        _state: Rc<RefCell<State>>,
        _append_nodes: bool,
    ) -> Self {
        let mut dom = Self::default(_state, props);
        if !_append_nodes {
            dom._append_nodes = false;
            dom.mount.insert(
                0,
                "let parent = target.parent_node().unwrap_throw();".to_string(),
            );
        }
        dom.generate_elements("target", elements, details);
        if _append_nodes {
            dom.update
                .push("self.state.borrow_mut().updated_idents.clear();".to_string());
        }
        // _state.callbacks.append(&mut dom.binded_callbacks);
        dom
    }

    fn add_event_listener(
        &mut self,
        element_name: &str,
        event_name: &str,
        position_in_callbacks: usize,
    ) {
        self.mount_mounted.push(format!(
            r#"add_listener!(self.{}, "{}", self.state, {})?;"#,
            element_name, event_name, position_in_callbacks
        ));
        self.drop.push(format!(
            r#"remove_listener!(self.{}, "{}", self.state, {}).unwrap_throw();"#,
            element_name, event_name, position_in_callbacks
        ));
    }

    fn generate_attributes(
        &mut self,
        element_name: &str,
        attrs: &Vec<Attribute>,
        details: &LocalDetails,
    ) -> () {
        //! Generate code to create and delete dom attributes.
        //! Generate code to add and remove event listeners.
        for attr in attrs {
            let Attribute { name, expr, .. } = attr;
            let mut namespace = attr.namespace.clone();

            let (init_value, ident) = match expr {
                AttrExprType::String(text) => (text.to_string(), None),
                AttrExprType::Ident(ref ident) => (format_ident!(self.props, ident), Some(ident)),
                AttrExprType::Block(block) => {
                    block.brace_token.span.warning(format!(
                        "Attribute's Block is not handled yet. To implement it see `{}:{}`",
                        file!(),
                        line!()
                    ));
                    // Could use IdentModifier, to add .borrow() or borrow_mut()
                    continue;
                }
            };

            let ident_name = match ident {
                Some(ident) => ident.to_string(),
                None => {
                    // String are ignore for `namespace:` (see W001 in html/)
                    namespace = "".to_string();
                    "".to_string()
                }
            };

            // Bind special cases
            if element_name.starts_with("input") && namespace == "bind" && name == "value" {
                println!("attr input");
                let data = ClosureBindInput::new(
                    element_name,
                    ident_name.clone(),
                    details
                        .states
                        .iter()
                        .filter(|state| {
                            println!("ident {} & name {}", state.ident, ident_name);
                            state.ident.to_string() == ident_name
                        })
                        .next()
                        // Error should have been handled in crate::check, so it won't panic
                        .unwrap(),
                );
                let position_in_callbacks = self._state.borrow().callbacks.len();
                // let position_in_callbacks =
                //     details.events_closures.len() + self.binded_callbacks.len();
                self.add_event_listener(element_name, "change", position_in_callbacks);

                // self.binded_callbacks.push(data.callback_on_change);
                self._state
                    .borrow_mut()
                    .callbacks
                    .push(data.callback_on_change);
                self.update.push(data.update_value);
                self.mount.push(data.init_value);
            } else if namespace == "on" {
                println!("attr on");
                let position_in_callbacks = details
                    .events_closures
                    .iter()
                    .enumerate()
                    .filter(|(_, event_closure)| event_closure.ident.to_string() == ident_name)
                    .next()
                    // Error should have been handled in crate::check, so it won't panic
                    .unwrap()
                    .0;
                self.add_event_listener(element_name, name, position_in_callbacks);
            } else if namespace == "bind" {
                todo!(
                    "Namespace bind is not implemented (execpt for input). To implement it see `{}:{}`",
                    file!(), line!()
                );
            } else {
                match expr {
                    AttrExprType::String(_) => self.init.push(format!(
                        r#"{element_name}.set_attribute("{name}", "{init_value}")?;"#
                    )),
                    _ => self.mount.push(format!(
                        r#"self.{element_name}.set_attribute("{name}", "{init_value}")?;"#
                    )),
                }
            }
        }
    }

    fn generate_node_name(&mut self, name: &str) -> String {
        let count = self._tag_count.entry(name.to_string()).or_insert(0);
        *count += 1;
        format!("{name}_{count}")
    }

    fn append_node(&mut self, target: &str, node_name: &str) {
        let line: String;
        if target == "target" {
            if self._append_nodes {
                line = format!("{}.append_child(&self.{})?;", target, node_name);
            } else {
                line = format!("parent.insert_before(&self.{}, Some(target))?;", node_name);
            }
            self.drop.push(format!("self.{node_name}.remove();"));
        } else {
            line = format!("self.{}.append_child(&self.{})?;", target, node_name);
        }
        self.mount.push(line);
    }

    fn create_node(&mut self, target: &str, tag: &str, document_method: &str) -> String {
        let node_name = self.generate_node_name(tag);
        let type_ = match tag {
            "input" => "Input",
            "button" => "Button",
            _ => "web_sys::Element",
        };
        self.init.push(format!(
            r#"let {}: {} = document.{}("{}")?.dyn_into()?;"#,
            node_name, type_, document_method, tag
        ));
        self.fields.insert(node_name.clone(), type_.to_string());
        self.append_node(target, &node_name);
        node_name
    }

    fn create_empty_node(&mut self, target: &str) -> String {
        let node_name = self.generate_node_name("empty");
        self.init.push(format!(
            r#"let {node_name} = document.create_text_node("");"#
        ));
        self.fields.insert(node_name.clone(), "Text".to_string());
        self.append_node(target, &node_name);
        node_name
    }

    fn create_text_node(
        &mut self,
        target: &str,
        tag: &str,
        document_method: &str,
        text: &str,
        escaped: bool,
    ) -> String {
        let node_name = self.generate_node_name(tag);
        if escaped {
            self.init.push(format!(
                // funny ahaha ! Maybe, it should be even more escaped ?! What about 2**16 ?
                r########"let {} = document.{}(r#######"{}"#######);"########,
                node_name, document_method, text
            ));
        } else {
            self.init.push(format!(
                "let {} = document.{}({});",
                node_name, document_method, text
            ));
        }
        self.fields.insert(node_name.clone(), "Text".to_string());
        self.append_node(target, &node_name);
        node_name
    }

    fn generate_elements(
        &mut self,
        parent_name: &str,
        elements: &Vec<Element>,
        details: &LocalDetails,
    ) -> Vec<String> {
        // let mut previous_name: String = "".to_string();
        for el in elements {
            // let mut name: String = "".to_string();
            match el {
                Element::Classic(Classic {
                    name: element_name,
                    ref attrs,
                    ref children,
                }) => {
                    println!("elements generated : {}", element_name);
                    let generated_name =
                        self.create_node(parent_name, &element_name, "create_element");
                    self.generate_attributes(&generated_name, attrs, details);
                    self.generate_elements(&generated_name, children, details);
                }
                Element::Text(text) => {
                    self.create_text_node(parent_name, "text", "create_text_node", text, true);
                }
                Element::ExprElement(el_expr) => match el_expr {
                    ExprElement::Ident(ref ident) => {
                        let name = self.create_text_node(
                            parent_name,
                            "text",
                            "create_text_node",
                            &format!(r#"&format!("{{}}", {})"#, format_ident!(self.props, ident)),
                            false,
                        );
                        println!("FORMAT IDENT : {}", format_ident!(self.props, ident));
                        println!(
                            "PROPS contains : {}",
                            self.props.contains_key(&ident.to_string())
                        );
                        println!("PROPS : {:?}", self.props);
                        let update = format!(
                            r#"self.{}.set_text_content(Some({}));"#,
                            name,
                            &format!(
                                r#"&format!("{{}}", {})"#,
                                format_ident!(self.props, ident: self)
                            ),
                        );
                        self.update.push(update);
                    }
                    ExprElement::If { conditions } => {
                        for Condition { expr, children } in conditions {
                            // TODO create sub dom element to handle the condition.
                            todo!(
                                "Sub dom nodes (init / update / drop) : {:?} / {:?}",
                                expr,
                                children
                            )
                        }
                    }
                    ExprElement::For { expr, children } => {
                        self.generate_for_loop(parent_name, details, &expr, children)
                    }
                    _ => continue,
                },
                Element::Comment(comment) => {
                    self.create_text_node(parent_name, "text", "create_comment", comment, true);
                }
            }
            // previous_name = name;
        }
        vec!["".to_string()]
    }

    fn generate_for_loop(
        &mut self,
        parent_name: &str,
        details: &LocalDetails,
        expr: &ExprForLoop,
        children: &Vec<Element>,
    ) {
        // TODO create sub dom element to handle the loop.
        let name = self.generate_node_name("for_loop");

        match expr.pat {
            syn::Pat::Ident(syn::PatIdent {
                ref ident,
                mutability,
                ..
            }) => {
                println!("{} ; {}", ident, mutability.is_some());
                // let mut for_loop_init = expr.clone();
                let mut for_loop_mount = expr.clone();
                // let mut for_loop_update = expr.clone();
                let for_loop_with_state = expr.clone();
                let mut ident_modifier = details.get_ident_modifier("self.state");
                ident_modifier.visit_expr_mut(&mut *for_loop_mount.expr);
                let type_info = get_pat_bindings(
                    format!(
                        "{}\n{}",
                        details.block,
                        quote!(#for_loop_with_state).to_string(),
                    ),
                    // expr.for_token.span.source_file().path().file_name(),
                    "unknown_file_name".into(),
                    name.clone(),
                );
                println!("INFO {:#?}", type_info);

                let updated_name = ident_from(format!("updated_{name}"));
                let dom_name = ident_from(format!("dom_{name}"));
                let mod_name = ident_from(name.clone());

                self.fields
                    .insert(dom_name.to_string(), format!("Vec<{mod_name}::DOM>"));
                self.init.push(format!("let {dom_name} = Vec::new();"));
                self.fields
                    .insert(updated_name.to_string(), "bool".to_string());
                self.init.push(format!("let {updated_name} = false;"));
                let empty_after = ident_from(self.create_empty_node(parent_name));

                let used_state_idents: syn::ExprArray = syn::parse_str(
                    format!(
                        "[{}]",
                        (&ident_modifier.names_ref | &ident_modifier.names_refmut)
                            .iter()
                            .map(|ident| format!(r#""{}".to_string()"#, ident))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                    .as_str(),
                )
                .unwrap();
                // self.update.push(
                //     quote!(
                //         console::log_1(&format!("UPDATE IDENTS {:?}", self.state.borrow().updated_idents).into());
                //         if self.state.borrow().updated_idents.intersection(&HashSet::from(#used_state_idents)).count() > 0  {
                //             console::log_1(&"Update loop".into());
                //             self.#dom_name.clear();
                //             let mut i = 0;
                //             #for_loop_mount
                //         }
                //     )
                //     .to_string(),
                // );
                // self.update.push(
                //     quote!(
                //     for dom in self.#dom_name.iter_mut() {
                //         dom.update()?;
                //     }
                //     )
                //     .to_string(),
                // );
                let mut extractor = IdentExtractor::new();

                let mut strings: Vec<String> = Vec::new();
                strings.push("plop".to_string());
                for s in strings {
                    println!("{}", s);
                }
                for (i, plop) in (0..5u32).enumerate() {
                    let p = i + plop as usize;
                    println!("{}: {} // {}", i, plop, p);
                }

                extractor.visit_pat(&expr.pat);

                let loop_idents: Vec<syn::FieldValue> = extractor
                    .idents
                    .iter()
                    .map(|ident| syn::parse_quote!(#ident))
                    .collect();

                let loop_idents_name: Vec<String> = extractor
                    .idents
                    .iter()
                    .map(|ident| ident.to_string())
                    .collect();

                let props = type_info
                    .iter()
                    .filter(|binding| loop_idents_name.contains(&binding.name))
                    .map(|binding| (binding.name.clone(), binding.ty.clone()))
                    .collect::<HashMap<_, _>>();

                let update_props: Vec<syn::Stmt> = extractor
                    .idents
                    .iter()
                    .map(|ident| syn::parse_quote!(dom.props.#ident = #ident.clone();))
                    // .map(|ident| {
                    //     let type_ = props.get(&ident.to_string());
                    //     match type_ {
                    //         Some(type_) => {
                    //             if type_.starts_with('&') {
                    //                 syn::parse_quote!(dom.props.#ident = #ident.clone();)
                    //             } else {
                    //                 syn::parse_quote!(dom.props.#ident = #ident;)
                    //             }
                    //         },
                    //         None => {
                    //             for_loop_mount.for_token.span.error("Can't coerce type from for loop.");
                    //             panic!("Can't coerce type from for loop.");
                    //         }
                    //     }
                    // })
                    .collect();

                let init_props: Vec<syn::FieldValue> = extractor
                    .idents
                    .iter()
                    .map(|ident| syn::parse_quote!(#ident: #ident.clone()))
                    // .map(|ident| {
                    //     let type_ = props.get(&ident.to_string());
                    //     match type_ {
                    //         Some(t) => {
                    //             if t.starts_with('&') {
                    //                 syn::parse_quote!(#ident)
                    //             } else {
                    //                 syn::parse_quote!(#ident: #ident.clone())
                    //             }
                    //         }
                    //         None => {
                    //             for_loop_mount
                    //                 .for_token
                    //                 .span
                    //                 .error("Can't coerce type from for loop.");
                    //             panic!("Can't coerce type from for loop.");
                    //         }
                    //     }
                    // })
                    .collect();

                for_loop_mount.body = syn::parse_quote!(
                {
                    i += 1;
                    let mut dom_instance = #mod_name::DOM::from_state(self.state.clone(), i, #mod_name::Props{ #(#init_props,)* })?;
                    dom_instance.mount(self.#empty_after.unchecked_ref())?;
                    self.#dom_name.push(dom_instance);
                }
                );

                let for_expr = &for_loop_mount.expr;
                self.mount.push(
                    quote!(
                    let mut i = 0;
                    #for_loop_mount
                    )
                    .to_string(),
                );

                self.update.push(
                    quote!(
                        // for dom in self.#dom_name.iter_mut() {
                        //     dom.update()?;
                        // }

                        // ForLoop's expr idents and ForLoop's block could be differentiate for optimization :
                        // - When only expr has changed with no props, we don't need to run dom.update()
                        // - When block has changed, we don't need to update props only to update
                        if self.state.borrow().updated_idents.intersection(&HashSet::from(#used_state_idents)).count() > 0  {
                            let mut new_instances = Vec::new();
                            let mut truncate_index = None;
                            #[allow(unused_parens)]
                            for (i, pair) in (#for_expr).zip_longest(self.#dom_name.iter_mut()).enumerate()
                            {
                                match pair {
                                    #[allow(unused_parens)]
                                    EitherOrBoth::Both((#(#loop_idents),*), dom) => {
                                        #(#update_props)*
                                        dom.update()?;
                                    }
                                    #[allow(unused_parens)]
                                    EitherOrBoth::Left((#(#loop_idents),*)) => {
                                        let props = #mod_name::Props { #(#init_props,)* };
                                        let mut dom_instance = #mod_name::DOM::from_state(self.state.clone(), i as u32, props)?;
                                        dom_instance.mount(self.#empty_after.unchecked_ref())?;
                                        new_instances.push(dom_instance)
                                    }
                                    EitherOrBoth::Right(_) => {
                                        truncate_index = Some(i);
                                        break;
                                    }
                                }
                            }
                            match truncate_index {
                                Some(index) => self.#dom_name.truncate(index as usize),
                                None => self.#dom_name.append(&mut new_instances),
                            }
                        }
                    )
                    .to_string(),
                );

                println!("Children in for loop are :\n{:#?}", children);
                let sub_dom = Dom::generate(children, details, props, self._state.clone(), false);
                // sub_dom.props.extend(
                //     props
                //         .iter()
                //         .map(|(f, type_)| {
                //             if type_.starts_with('&') {
                //                 (f.clone(), type_.clone())
                //             } else {
                //                 (f.clone(), type_.clone())
                //             }
                //         })
                //         .collect::<Vec<_>>(),
                // );
                // let mut dom = Dom::default(self._state.clone());
                // dom._append_nodes = false;
                // dom.generate_elements("target", children, details);
                self.sub_doms.insert(name.clone(), sub_dom);
            }
            _ => {
                expr.for_token.span.warning(format!("For loop is useless, for now only Pat::Ident are supported. To implement another Pat see {}:{}", file!(), line!()));
                return;
            }
        };
    }
}

fn ident_from(name: String) -> syn::Ident {
    syn::parse_str(name.as_str()).unwrap()
}
