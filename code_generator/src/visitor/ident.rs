use std::collections::HashSet;

use proc_macro2_diagnostics::SpanDiagnosticExt;
use quote::quote;
use syn::{
    parse_str,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
    Error, Expr, ExprAssign, ExprAssignOp, ExprBinary, ExprMacro, ExprMethodCall, ExprPath, Ident,
    Local, Result,
};

pub struct IdentExtractor {
    idents: Vec<Ident>,
}

impl IdentExtractor {
    fn new() -> Self {
        Self { idents: Vec::new() }
    }

    fn names(&self) -> HashSet<String> {
        self.idents.iter().map(|ident| ident.to_string()).collect()
    }

    fn intersect_names(&self, names: &HashSet<String>) -> HashSet<String> {
        self.names().intersection(names).cloned().collect()
    }

    #[allow(dead_code)]
    fn ident_from_intersect_names(&self, names: &HashSet<String>) -> HashSet<String> {
        self.names().intersection(names).cloned().collect()
    }
}

impl<'ast> Visit<'ast> for IdentExtractor {
    fn visit_ident(&mut self, node: &'ast Ident) {
        self.idents.push(node.clone());
        visit::visit_ident(self, node);
    }
}

pub struct IdentModifier {
    pub state_names: HashSet<String>,
    pub names: HashSet<String>,
    pub names_refmut: HashSet<String>,
    pub names_ref: HashSet<String>,
    pub locals: HashSet<String>,
    pub errors: Vec<Error>,
    pub count_expr_path: usize,
    state_ident: String,
}

impl IdentModifier {
    pub fn new(state_names: HashSet<String>, state_ident: String) -> Self {
        Self {
            state_names,
            names: HashSet::new(),
            names_refmut: HashSet::new(),
            names_ref: HashSet::new(),
            locals: HashSet::new(),
            errors: Vec::new(),
            count_expr_path: 0,
            state_ident,
        }
    }

    #[allow(dead_code)]
    pub fn moved_names(&self) -> HashSet<String> {
        self.names.difference(&self.locals).cloned().collect()
    }

    #[allow(dead_code)]
    pub fn raise_errors(&self) -> Result<()> {
        if self.errors.len() > 0 {
            let mut error = self.errors[0].clone();
            for e in self.errors.iter().skip(1) {
                error.combine(e.clone());
            }
            Err(error)?;
        }
        Ok(())
    }

    fn replace_expr(&mut self, node: &mut Box<Expr>, prefix: &str, suffix: &str) {
        let mut visitor = IdentExtractor::new();
        visitor.visit_expr(&node);
        let names = visitor.intersect_names(&self.state_names);
        if names.len() == 1 && visitor.idents.len() == 1 {
            let to_parse = format!("{}{}{}", prefix, quote!(#node).to_string(), suffix);
            self.try_parse_node(node, to_parse, names);
            // match parse_str::<Expr>(&format!(
            //     "{}{}{}",
            //     prefix,
            //     quote!(#node).to_string(),
            //     suffix
            // )) {
            //     Ok(new_node) => {
            //         self.modified.extend(names);
            //         **node = new_node;
            //     }
            //     Err(err) => {
            //         err.span().error(err.to_string());
            //         self.errors.push(err);
            //     }
            // }
        } else if names.len() == 1 {
            // TODO once implemented do not forget to add it to modified names
            // self.modified.extend(names);
            let span = visitor
                .idents
                .iter()
                .filter(|ident| ident.to_string() == *names.iter().next().unwrap())
                .next()
                .unwrap()
                .span();
            let msg = format!(
                "The left side of ExprAssignOp with multiple ident is not handle.\
                    If there's an use case : see `{}:{}` to implement it.",
                file!(),
                line!()
            );
            span.error(msg.clone());
            self.errors.push(Error::new(span, msg));
        }
    }

    fn try_parse_node<P>(&mut self, node: &mut P, to_parse: String, name: HashSet<String>)
    where
        P: syn::parse::Parse,
    {
        match parse_str(to_parse.as_str()) {
            Ok(new_node) => {
                if to_parse.contains("borrow_mut") {
                    self.names_refmut.extend(name);
                } else if to_parse.contains("borrow") {
                    self.names_ref.extend(name);
                }
                *node = new_node;
            }
            Err(err) => {
                err.span().error(err.to_string());
                self.errors.push(err);
            }
        }
    }
}

impl VisitMut for IdentModifier {
    fn visit_local_mut(&mut self, node: &mut Local) {
        match node {
            Local {
                pat: syn::Pat::Ident(syn::PatIdent { ident, .. }),
                ..
            } => self.locals.insert(ident.to_string()),
            _ => false,
        };
        visit_mut::visit_local_mut(self, node);
    }

    fn visit_expr_binary_mut(&mut self, node: &mut ExprBinary) {
        let borrow = format!("{}.borrow().", self.state_ident);
        match *node.right {
            Expr::Binary(_) => (),
            _ => self.replace_expr(&mut node.left, borrow.as_str(), ""),
        }
        self.replace_expr(&mut node.right, borrow.as_str(), "");
        visit_mut::visit_expr_binary_mut(self, node);
    }

    fn visit_expr_assign_op_mut(&mut self, node: &mut ExprAssignOp) {
        let borrow = format!("{}.borrow().", self.state_ident);
        let borrow_mut = format!("{}.borrow_mut().", self.state_ident);
        self.replace_expr(&mut node.left, borrow_mut.as_str(), "");
        match *node.right {
            Expr::Binary(_) => (),
            _ => self.replace_expr(&mut node.right, borrow.as_str(), ""),
        }
        visit_mut::visit_expr_assign_op_mut(self, node);
    }

    fn visit_expr_assign_mut(&mut self, node: &mut ExprAssign) {
        let borrow = format!("{}.borrow().", self.state_ident);
        let borrow_mut = format!("{}.borrow_mut().", self.state_ident);
        self.replace_expr(&mut node.left, borrow_mut.as_str(), "");
        match *node.right {
            Expr::Binary(_) => (),
            _ => self.replace_expr(&mut node.right, borrow.as_str(), ""),
        }
        visit_mut::visit_expr_assign_mut(self, node);
    }

    fn visit_expr_method_call_mut(&mut self, node: &mut ExprMethodCall) {
        let borrow_mut = format!("{}.borrow_mut().", self.state_ident);
        match *node.receiver {
            Expr::Path(ExprPath { ref path, .. }) => {
                let name = path.segments[0].ident.to_string();
                if self.state_names.contains(&name) {
                    let to_parse = format!("{borrow_mut}{}", quote!(#node));
                    // self.try_parse_node::<ExprMethodCall, _>(node, to_parse, name);
                    self.try_parse_node(node, to_parse, HashSet::from([name]));
                    // match parse_str(to_parse.as_str()) {
                    //     Ok(new_node) => {
                    //         self.modified.insert(name);
                    //         *node = new_node;
                    //     }
                    //     Err(err) => {
                    //         err.span().error(err.to_string());
                    //         self.errors.push(err);
                    //     }
                    // }
                }
            }
            _ => (),
        }
        visit_mut::visit_expr_method_call_mut(self, node);
    }

    fn visit_expr_mut(&mut self, node: &mut Expr) {
        let borrow = format!("{}.borrow().", self.state_ident);
        match *node {
            Expr::Path(ExprPath { ref path, .. }) => {
                let name = path.segments[0].ident.to_string();
                if self.state_names.contains(&name) {
                    let to_parse = format!("{borrow}{}", quote!(#node));
                    self.try_parse_node(node, to_parse, HashSet::from([name]));
                }
            }
            _ => (),
        }
        visit_mut::visit_expr_mut(self, node);
    }

    fn visit_expr_path_mut(&mut self, node: &mut ExprPath) {
        self.count_expr_path += 1;
        visit_mut::visit_expr_path_mut(self, node);
    }

    fn visit_expr_macro_mut(&mut self, node: &mut ExprMacro) {
        let mut visitor = IdentExtractor::new();
        visitor.visit_path(&node.mac.path);
        let msg = "Macro are not implemented yet in html.";
        let span = visitor.idents[0].span();
        span.error(msg);
        self.errors.push(Error::new(span, msg));
        // Delegate to the default impl to visit any nested functions.
        visit_mut::visit_expr_macro_mut(self, node);
    }

    fn visit_ident_mut(&mut self, node: &mut Ident) {
        // println!("Ident with name={}", node.to_string());
        self.names.insert(node.to_string());

        // Delegate to the default impl to visit any nested functions.
        visit_mut::visit_ident_mut(self, node);
    }
}

#[cfg(test)]
mod tests {
    use super::{IdentModifier, VisitMut};
    use quote::quote;
    use std::collections::HashSet;

    #[test]
    fn impl_display_for_block() {
        fn assert_impl_display<T: ?Sized + std::fmt::Display>() {}
        assert_impl_display::<&str>();
    }

    #[test]
    fn plop() -> syn::Result<()> {
        // let block = r#"{plop}"#;
        let block = r#"
            {
            let closure = |number| counter += 5 + plop;
            let closure = |number| counter + 5;
            }

        "#;
        // let block = r#""#;
        // let block = r#""#;
        let mut block_user: syn::Block = syn::parse_str(block)?;
        println!("{:#?}", block_user);
        let mut ident_visitor = IdentModifier::new(
            HashSet::from(["counter".to_string(), "plop".to_string()]),
            "s".to_string(),
        );
        ident_visitor.visit_block_mut(&mut block_user);
        // assert!(ident_visitor.raise_errors().is_err());
        // println!("\nFOund names {:?}", ident_visitor.names);
        // println!("Moved names {:?}\n", ident_visitor.moved_names());
        println!(
            "{}",
            quote!(#block_user)
                .to_string()
                .split(";")
                .collect::<Vec<&str>>()
                .join(";\n")
        );
        assert!(false);
        Ok(())
    }
}
