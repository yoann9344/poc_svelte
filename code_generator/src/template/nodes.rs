use std::collections::HashMap;

use proc_macro2_diagnostics::SpanDiagnosticExt;

use crate::{
    html::{AttrExprType, Attribute, Classic, Condition, Element, ExprElement},
    state_block::LocalDetails,
};

use super::ClosureBindInput;

#[derive(Default)]
pub struct Dom {
    pub init: Vec<String>,
    pub mount: Vec<String>,
    pub mount_mounted: Vec<String>,
    pub update: Vec<String>,
    pub drop: Vec<String>,
    pub binded_callbacks: Vec<(String, String)>,
    pub dom_state: Vec<(String, String)>,
    tag_count: HashMap<String, usize>,
}

impl Dom {
    pub fn generate(elements: &Vec<Element>, details: &LocalDetails) -> Self {
        let mut dom = Self::default();
        dom.generate_elements("parent", elements, details);
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
                let position_in_callbacks =
                    details.events_closures.len() + self.binded_callbacks.len();
                self.add_event_listener(element_name, "change", position_in_callbacks);

                self.binded_callbacks.push(data.callback_on_change);
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
        let count = self.tag_count.entry(name.to_string()).or_insert(0);
        *count += 1;
        format!("{name}_{count}")
    }

    fn append_node(&mut self, parent: &str, node_name: &str) {
        if parent == "parent" {
            self.mount
                .push(format!("{}.append_child(&self.{})?;", parent, node_name));
        } else {
            self.mount.push(format!(
                "self.{}.append_child(&self.{})?;",
                parent, node_name
            ));
        }
    }

    fn create_node(&mut self, parent: &str, tag: &str, document_method: &str) -> String {
        let node_name = self.generate_node_name(tag);
        let type_ = match tag {
            "input" => "Input",
            "button" => "Button",
            _ => "Element",
        };
        self.init.push(format!(
            r#"let {}: {} = document.{}("{}")?.dyn_into()?;"#,
            node_name, type_, document_method, tag
        ));
        self.dom_state.push((node_name.clone(), type_.to_string()));
        self.append_node(parent, &node_name);
        node_name
    }

    fn create_text_node(
        &mut self,
        parent: &str,
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
        self.dom_state.push((node_name.clone(), "Text".to_string()));
        self.append_node(parent, &node_name);
        node_name
    }

    fn generate_elements(
        &mut self,
        parent_name: &str,
        elements: &Vec<Element>,
        details: &LocalDetails,
    ) -> Vec<String> {
        for el in elements {
            match el {
                Element::Classic(Classic {
                    name,
                    ref attrs,
                    ref children,
                }) => {
                    let generated_name = self.create_node(parent_name, &name, "create_element");
                    self.generate_attributes(&generated_name, attrs, details);
                    self.generate_elements(&generated_name, children, details);
                }
                Element::Text(text) => {
                    self.create_text_node(parent_name, "text", "create_text_node", text, true);
                }
                Element::ExprElement(el_expr) => match el_expr {
                    ExprElement::Ident(ref ident) => {
                        let generated_name = self.create_text_node(
                            parent_name,
                            "text",
                            "create_text_node",
                            &format!(r#"&format!("{{}}", state.borrow().{})"#, ident),
                            false,
                        );
                        let update = format!(
                            r#"self.{}.set_text_content(Some({}));"#,
                            generated_name,
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
                    _ => (),
                },
                Element::Comment(comment) => {
                    self.create_text_node(parent_name, "text", "create_comment", comment, true);
                }
            }
        }
        vec!["".to_string()]
    }
}
