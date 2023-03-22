use std::collections::HashSet;

use quote::quote;
use syn::{
    parse_str,
    visit::{self, Visit},
    visit_mut::{self, VisitMut},
    Error, Expr, ExprAssignOp, ExprBinary, ExprMacro, ExprPath, Ident, Local, Result,
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
    pub locals: HashSet<String>,
    pub errors: Vec<Error>,
    pub count_expr_path: usize,
}

impl IdentModifier {
    pub fn new(state_names: HashSet<String>) -> Self {
        Self {
            state_names,
            names: HashSet::new(),
            locals: HashSet::new(),
            errors: Vec::new(),
            count_expr_path: 0,
        }
    }

    pub fn moved_names(&self) -> HashSet<String> {
        self.names.difference(&self.locals).cloned().collect()
    }

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
            match parse_str::<Expr>(&format!(
                "{}{}{}",
                prefix,
                quote!(#node).to_string(),
                suffix
            )) {
                Ok(new_node) => **node = new_node,
                Err(err) => self.errors.push(err),
            }
        } else if names.len() == 1 {
            let span = visitor
                .idents
                .iter()
                .filter(|ident| ident.to_string() == *names.iter().next().unwrap())
                .next()
                .unwrap()
                .span();
            self.errors.push(Error::new(
                span,
                format!(
                    "The left side of ExprAssignOp with multiple ident is not handle.\
                    If there's an use case : see `{}:{}` to implement it.",
                    file!(),
                    line!()
                ),
            ));
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
        match *node.right {
            Expr::Binary(_) => (),
            _ => self.replace_expr(&mut node.left, "s.borrow().", ""),
        }
        self.replace_expr(&mut node.right, "s.borrow().", "");
        visit_mut::visit_expr_binary_mut(self, node);
    }

    fn visit_expr_assign_op_mut(&mut self, node: &mut ExprAssignOp) {
        self.replace_expr(&mut node.left, "s.borrow_mut().", "");
        match *node.right {
            Expr::Binary(_) => (),
            _ => self.replace_expr(&mut node.right, "s.borrow().", ""),
        }
        visit_mut::visit_expr_assign_op_mut(self, node);
    }

    fn visit_expr_path_mut(&mut self, node: &mut ExprPath) {
        self.count_expr_path += 1;
        visit_mut::visit_expr_path_mut(self, node);
    }

    fn visit_expr_macro_mut(&mut self, node: &mut ExprMacro) {
        let mut visitor = IdentExtractor::new();
        visitor.visit_path(&node.mac.path);
        self.errors.push(Error::new(
            visitor.idents[0].span(),
            "Macro are not implemented yet in html.",
        ));
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
        let mut ident_visitor =
            IdentModifier::new(HashSet::from(["counter".to_string(), "plop".to_string()]));
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
