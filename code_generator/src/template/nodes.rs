use std::collections::HashMap;
use std::{cell::RefCell, rc::Rc};

use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::visit_mut::VisitMut;

use crate::{
    html::{AttrExprType, Attribute, Classic, Condition, Element, ExprElement},
    state_block::LocalDetails,
};
pub use sailfish::TemplateOnce;

use super::ClosureBindInput;

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
    pub fields: Vec<(String, String)>,
    pub sub_doms: HashMap<String, Dom>,
    // data not used in template (most start with _)
    pub _state: Rc<RefCell<State>>,
    _tag_count: HashMap<String, usize>,
    _append_nodes: bool,
}

impl Dom {
    fn default(_state: Rc<RefCell<State>>) -> Self {
        Self {
            init: Vec::new(),
            mount: Vec::new(),
            mount_mounted: Vec::new(),
            update: Vec::new(),
            drop: Vec::new(),
            fields: Vec::new(),
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
        _state: Rc<RefCell<State>>,
        _append_nodes: bool,
    ) -> Self {
        let mut dom = Self::default(_state);
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
                AttrExprType::Ident(ref ident) => (format!("state.borrow().{ident}"), Some(ident)),
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
        self.fields.push((node_name.clone(), type_.to_string()));
        self.append_node(target, &node_name);
        node_name
    }

    fn create_empty_node(&mut self, target: &str) -> String {
        let node_name = self.generate_node_name("empty");
        self.init.push(format!(
            r#"let {node_name} = document.create_text_node("");"#
        ));
        self.fields.push((node_name.clone(), "Text".to_string()));
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
        self.fields.push((node_name.clone(), "Text".to_string()));
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
                            &format!(r#"&format!("{{}}", state.borrow().{})"#, ident),
                            false,
                        );
                        let update = format!(
                            r#"self.{}.set_text_content(Some({}));"#,
                            name,
                            &format!(r#"&format!("{{}}", self.state.borrow().{})"#, ident),
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
                        // TODO create sub dom element to handle the loop.
                        let name = self.generate_node_name("for_loop");
                        // let dom_id = self._tag_count.get("for_loop");
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
                                let mut ident_modifier = details.get_ident_modifier("self.state");
                                ident_modifier.visit_expr_mut(&mut *for_loop_mount.expr);

                                let updated_name = ident_from(format!("updated_{name}"));
                                let dom_name = ident_from(format!("dom_{name}"));
                                let mod_name = ident_from(name.clone());

                                self.fields
                                    .push((dom_name.to_string(), format!("Vec<{mod_name}::DOM>")));
                                self.init.push(format!("let {dom_name} = Vec::new();"));
                                self.fields
                                    .push((updated_name.to_string(), "bool".to_string()));
                                self.init.push(format!("let {updated_name} = false;"));
                                let empty_after = ident_from(self.create_empty_node(parent_name));

                                // // TODO: Must use insertBefore instead to use node previous_name
                                // let parent_ident = if previous_name == "" || true {
                                //     proc_macro2::Ident::new(
                                //         parent_name,
                                //         proc_macro2::Span::call_site(),
                                //     )
                                // } else {
                                //     proc_macro2::Ident::new(
                                //         previous_name.as_str(),
                                //         proc_macro2::Span::call_site(),
                                //     )
                                // };
                                for_loop_mount.body = syn::parse_quote!(
                                    {
                                        i += 1;
                                        let mut dom_instance = #mod_name::DOM::from_state(self.state.clone(), i)?;
                                        dom_instance.mount(self.#empty_after.unchecked_ref())?;
                                        self.#dom_name.push(dom_instance);
                                    }
                                );
                                self.mount.push(
                                    quote::quote!(
                                        let mut i = 0;
                                        #for_loop_mount
                                    )
                                    .to_string(),
                                );
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
                                self.update.push(
                                    quote::quote!(
                                        console::log_1(&format!("UPDATE IDENTS {:?}", self.state.borrow().updated_idents).into());
                                        if self.state.borrow().updated_idents.intersection(&HashSet::from(#used_state_idents)).count() > 0  {
                                            console::log_1(&"Update loop".into());
                                            self.#dom_name.clear();
                                            let mut i = 0;
                                            #for_loop_mount
                                        }
                                    )
                                    .to_string(),
                                );
                                self.update.push(
                                    quote::quote!(
                                        for dom in self.#dom_name.iter_mut() {
                                            dom.update()?;
                                        }
                                    )
                                    .to_string(),
                                );
                                expr.for_token.span.warning(format!("For loop is useless, for now only Pat::Ident are supported. To implement another Pat see {}:{}", file!(), line!()))
                            }
                            _ => {
                                expr.for_token.span.warning(format!("For loop is useless, for now only Pat::Ident are supported. To implement another Pat see {}:{}", file!(), line!()));
                                continue;
                            }
                        };
                        println!("Children in for loop are :\n{:#?}", children);
                        let sub_dom = Dom::generate(children, details, self._state.clone(), false);
                        // let mut dom = Dom::default(self._state.clone());
                        // dom._append_nodes = false;
                        // dom.generate_elements("target", children, details);
                        self.sub_doms.insert(name.clone(), sub_dom);
                        println!("OUT OF LOOP");
                        // todo!(
                        //     "Sub dom nodes (init / update / drop) : {:?} / {:?}",
                        //     expr,
                        //     children
                        // )
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
}

fn ident_from(name: String) -> syn::Ident {
    syn::parse_str(name.as_str()).unwrap()
}
