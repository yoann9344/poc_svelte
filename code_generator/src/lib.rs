extern crate proc_macro;
use proc_macro::TokenStream;
use quote::ToTokens;
use syn::*;

mod check;
mod html;
mod state_block;
mod template;
mod visitor;
// use template::{block_from_templates, Imports, Macros, TemplateOnce};
use template::Component;

use crate::{
    check::check_ident_expr,
    html::{Element, Root},
    state_block::extract_locals,
};

#[proc_macro]
pub fn full(block: TokenStream) -> TokenStream {
    println!("Start parsing");
    match full_error_wrapper(block.into()) {
        Ok(token_stream) => {
            println!("Match sucess\n\n\n{}", token_stream.to_string());
            token_stream
        }
        Err(parsing_error) => {
            println!("Match error");
            parsing_error.to_compile_error().into()
        }
    }
}

struct Full {
    block: Block,
    elements: Vec<Element>,
}

impl syn::parse::Parse for Full {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let block: Block = input.parse()?;
        println!("{:#?}", block);
        let elements: Vec<Element> = input.parse::<Root>()?.0;
        println!("ELEMENTS {:#?}", elements);
        Ok(Self { block, elements })
    }
}

fn full_error_wrapper(input: proc_macro2::TokenStream) -> Result<TokenStream> {
    let Full { block, elements } = parse2(input)?;
    let mut details_locals = extract_locals(&block)?;
    check_ident_expr(&elements, &details_locals)?;
    Ok(Component::new(&mut details_locals, &elements).to_token_stream())
}

#[proc_macro]
pub fn make_answer(item: TokenStream) -> TokenStream {
    let mut block_user: syn::Block = parse(item).unwrap();
    println!("{:#?}", block_user);
    // .for_each(|statement| println!("{:#?}", statement));
    let local = block_user
        .stmts
        .iter()
        .filter_map(|statement| match statement {
            syn::Stmt::Local(local) => Some(local),
            _ => None,
        })
        .cloned()
        .collect::<Vec<syn::Local>>()[0]
        .clone();

    // Add return ident
    let name = match local {
        Local {
            pat: Pat::Ident(PatIdent { ident: name, .. }),
            ..
        } => name.to_string(),
        _ => panic!(),
    };
    println!("Local => {:?}", name);
    // let expr_path: Expr = format!("{}", name).parse().unwrap();
    // let mut fn_template: syn::Item = parse(fn_template).unwrap();
    // block.stmts.push()

    // put block_user into a function
    let fn_template: TokenStream = format!("fn answer() -> u32 {{ let {} = 33; {} }}", name, name)
        .parse()
        .unwrap();
    let mut fn_template: syn::Item = parse(fn_template).unwrap();
    println!("Fn => {:#?}", fn_template);
    match fn_template {
        Item::Fn(ItemFn { ref block, .. }) => {
            block_user.stmts.push(block.stmts.last().unwrap().clone())
        }

        _ => panic!(),
    }
    match fn_template {
        Item::Fn(ref mut item_fn) => item_fn.block = Box::new(block_user.clone()),
        _ => panic!(),
    }
    println!("Fn => {:#?}", fn_template);
    fn_template.clone().to_token_stream().into()

    // for i in ast.iter {
    //     println!("Item « {} »", i)
    // }
    // let function = format!("fn answer(a: u32) -> u32 {{ {} }}", item);
    // function.parse().unwrap()
    // "fn answer() -> u32 {  }".parse().unwrap()
    // "fn answer() -> u32 { 42 }".parse().unwrap()
}
// extern crate proc_macro;
// use proc_macro::TokenStream;
// use syn::*;

// mod tree;
// mod visitors;

// use crate::tree::{Element, Root};

// fn html_error_wrapper(block: proc_macro2::TokenStream) -> Result<TokenStream> {
//     // let element: Element = parse2(block)?;
//     let elements: Vec<Element> = parse2::<Root>(block)?.0;
//     println!("{:#?}", elements);
//     Ok("fn main() { println!(\"Success Method\"); }"
//         .parse()
//         .unwrap())
// }

// // #[proc_macro]
// // pub fn show_syn(block: TokenStream) -> TokenStream {
// //     let block_user: Block = parse(block).unwrap();
// //     println!("{:#?}", block_user);
// //     "fn truc() { println!(\"Success Method\"); }"
// //         .parse()
// //         .unwrap()
// // }

// #[proc_macro]
// pub fn html(block: TokenStream) -> TokenStream {
//     println!("Start parsing");
//     match html_error_wrapper(block.into()) {
//         Ok(html_root) => {
//             println!("Match sucess");
//             html_root
//         }
//         Err(parsing_error) => {
//             println!("MAtch error");
//             parsing_error.to_compile_error().into()
//         }
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::tree::{AttrExprType, Attribute, Classic, Condition, Element, ExprElement};
//     use quote::quote;

//     #[test]
//     fn element_classic() -> syn::Result<()> {
//         let html = r#"
//             <plop>
//                 <truc></truc>
//                 <other></other>
//                 <void/>
//                 <!--other></other-->
//             </plop>
//             "#;

//         let el: Element = syn::parse_str(html)?;
//         match el {
//             Element::Classic(Classic { name, children, .. }) => {
//                 assert_eq!(name, "plop");
//                 assert_eq!(children.len(), 4);
//                 match children.as_slice() {
//                     [Element::Classic(Classic {
//                         name: name_1,
//                         attrs,
//                         ..
//                     }), Element::Classic(Classic { name: name_2, .. }), Element::Classic(Classic { name: name_3, .. }), Element::Comment(comment)] =>
//                     {
//                         assert_eq!(attrs.len(), 0);
//                         assert_eq!(name_1, "truc");
//                         assert_eq!(name_2, "other");
//                         assert_eq!(name_3, "void");
//                         assert_eq!(comment, "<!--other></other-->");
//                     }
//                     _ => assert!(false, "Children don't match"),
//                 }
//             }
//             _ => assert!(false, "Element does't match."),
//         }
//         Ok(())
//     }

//     #[test]
//     fn element_if() -> syn::Result<()> {
//         let html = r#"
//             <ul>
//                 {if variable}
//                     <li>if</li>
//                 {else if plop == 10}
//                     <li>else if</li>
//                 {else}
//                     <li>if</li>
//                 {/if}
//             </ul>
//             "#;

//         let el: Element = syn::parse_str(html)?;
//         match el {
//             Element::Classic(Classic { name, children, .. }) => {
//                 assert_eq!(name, "ul");
//                 assert_eq!(children.len(), 1);
//                 match children.as_slice() {
//                     [Element::ExprElement(ExprElement::If { conditions })] => {
//                         assert_eq!(conditions.len(), 3);
//                         match conditions.as_slice() {
//                             [Condition {
//                                 expr: syn::Expr::Path(path),
//                                 children: children_1,
//                             }, Condition {
//                                 expr: syn::Expr::Binary(binary),
//                                 children: children_2,
//                             }, Condition {
//                                 expr: syn::Expr::Lit(lit),
//                                 children: children_3,
//                             }] => {
//                                 assert_eq!(quote!(#path).to_string(), "variable");
//                                 assert_eq!(quote!(#binary).to_string(), "plop == 10");
//                                 assert_eq!(quote!(#lit).to_string(), "true");
//                                 assert_eq!(children_1.len(), 1);
//                                 assert_eq!(children_2.len(), 1);
//                                 assert_eq!(children_3.len(), 1);
//                                 assert_ne!(children_1[0], children_2[0]);
//                                 assert_eq!(children_1[0], children_3[0]);
//                             }
//                             _ => assert!(false, "Children don't match"),
//                         }
//                     }
//                     _ => assert!(false, "Children don't match"),
//                 }
//             }
//             _ => assert!(false, "Element does't match."),
//         }
//         Ok(())
//     }
//     #[test]
//     fn attribute_classic() -> syn::Result<()> {
//         let html = r#"
//             <plop
//                 attr="value"
//                 attr2=value2
//                 attr3
//                 bind:value={variable}
//                 on:click={counter += 1}>
//             </plop>
//             "#;

//         let el: Element = syn::parse_str(html)?;
//         match el {
//             Element::Classic(Classic { name, attrs, .. }) => {
//                 assert_eq!(name, "plop");
//                 match attrs.as_slice() {
//                     [Attribute {
//                         namespace: namespace_1,
//                         name: name_1,
//                         expr: AttrExprType::String(value_1),
//                     }, Attribute {
//                         namespace: namespace_2,
//                         name: name_2,
//                         expr: AttrExprType::String(value_2),
//                     }, Attribute {
//                         namespace: namespace_3,
//                         name: name_3,
//                         expr: AttrExprType::String(value_3),
//                     }, Attribute {
//                         namespace: namespace_4,
//                         name: name_4,
//                         expr: AttrExprType::Ident(value_4),
//                         // expr: AttrExprType::Block(value_4),
//                     }, Attribute {
//                         namespace: namespace_5,
//                         name: name_5,
//                         expr: AttrExprType::Block(value_5),
//                     }] => {
//                         assert_eq!(namespace_1, "");
//                         assert_eq!(name_1, "attr");
//                         assert_eq!(value_1, "value");

//                         assert_eq!(namespace_2, "");
//                         assert_eq!(name_2, "attr2");
//                         assert_eq!(value_2, "value2");

//                         assert_eq!(namespace_3, "");
//                         assert_eq!(name_3, "attr3");
//                         assert_eq!(value_3, "");

//                         assert_eq!(namespace_4, "bind");
//                         assert_eq!(name_4, "value");
//                         assert_eq!(value_4.to_string(), "variable");

//                         assert_eq!(namespace_5, "on");
//                         assert_eq!(name_5, "click");
//                         assert_eq!(quote!(#value_5).to_string(), "{ counter += 1 }");
//                         // assert!(false);
//                     }
//                     _ => assert!(false, "Children don't match"),
//                 }
//             }
//             _ => assert!(false, "Element does't match."),
//         }
//         Ok(())
//     }

//     // #[test]
//     // fn plop() -> syn::Result<()> {
//     //     // let block = r#"{plop}"#;
//     //     let block = r#"
//     //         {
//     //         println!("test {}", plop);truc; path; let inner = "plop"; inner;
//     //         let closure = |number| number + 5 + in_closure;
//     //         closure(inner);
//     //         {variable}
//     //         {let variable = 5;}
//     //         for i in plop {}
//     //         if plop && test == 10 {a}
//     //         if test {}
//     //         test == binary;
//     //         }

//     //     "#;
//     //     // let block = r#""#;
//     //     // let block = r#""#;
//     //     // let block_user: syn::Block = syn::parse_str(block)?;
//     //     // println!("{:#?}", block_user);

//     //     use quote::ToTokens;
//     //     use syn::{
//     //         braced,
//     //         parse::{Parse, ParseStream},
//     //     };
//     //     // let expr: syn::Expr = syn::parse_str("test")?;
//     //     // let expr: syn::Expr = syn::parse_str("test == binary")?;
//     //     struct MyForLoop {
//     //         expr: ExprForLoop,
//     //     }
//     //     use syn::{punctuated::Punctuated, *};
//     //     pub fn multi_pat_with_leading_vert(input: ParseStream) -> Result<Pat> {
//     //         let leading_vert: Option<Token![|]> = input.parse()?;
//     //         multi_pat_impl(input, leading_vert)
//     //     }

//     //     fn multi_pat_impl(input: ParseStream, leading_vert: Option<Token![|]>) -> Result<Pat> {
//     //         let mut pat: Pat = input.parse()?;
//     //         if leading_vert.is_some()
//     //             || input.peek(Token![|]) && !input.peek(Token![||]) && !input.peek(Token![|=])
//     //         {
//     //             let mut cases = Punctuated::new();
//     //             cases.push_value(pat);
//     //             while input.peek(Token![|]) && !input.peek(Token![||]) && !input.peek(Token![|=]) {
//     //                 let punct = input.parse()?;
//     //                 cases.push_punct(punct);
//     //                 let pat: Pat = input.parse()?;
//     //                 cases.push_value(pat);
//     //             }
//     //             pat = Pat::Or(PatOr {
//     //                 attrs: Vec::new(),
//     //                 leading_vert,
//     //                 cases,
//     //             });
//     //         }
//     //         Ok(pat)
//     //     }
//     //     impl Parse for MyForLoop {
//     //         fn parse(braced_input: ParseStream) -> syn::Result<Self> {
//     //             println!("Start to parse");
//     //             let input;
//     //             braced!(input in braced_input);
//     //             let attrs = input.call(Attribute::parse_outer)?;
//     //             let label: Option<Label> = input.parse()?;
//     //             println!("Start to parse");
//     //             let for_token: Token![for] = input.parse()?;

//     //             println!("Start to parse");
//     //             let pat = multi_pat_with_leading_vert(&input)?;

//     //             let in_token: Token![in] = input.parse()?;
//     //             println!("without eager");
//     //             let expr: Expr = input.call(Expr::parse_without_eager_brace)?;
//     //             println!("Block parsed");
//     //             let body: Block = parse_str("{  }")?;
//     //             println!("create Loop");
//     //             let for_loop = ExprForLoop {
//     //                 attrs,
//     //                 label,
//     //                 for_token,
//     //                 pat,
//     //                 in_token,
//     //                 expr: Box::new(expr),
//     //                 body,
//     //             };
//     //             println!("Loop created");
//     //             println!("{:#?}", for_loop);
//     //             Ok(MyForLoop { expr: for_loop })
//     //         }
//     //     }
//     //     struct MyIf {
//     //         expr: Expr,
//     //     }
//     //     impl Parse for MyIf {
//     //         fn parse(braced_input: ParseStream) -> syn::Result<Self> {
//     //             let input;
//     //             braced!(input in braced_input);
//     //             input.parse::<Token![if]>()?;
//     //             Ok(Self {
//     //                 expr: input.call(Expr::parse_without_eager_brace)?,
//     //             })
//     //         }
//     //     }
//     //     // let expr: MyForLoop = parse_str("{for i in 0..10}")?;
//     //     let expr: MyIf = parse_str("{if plo}")?;
//     //     println!("Loop parsed");
//     //     // let expr: syn::Expr = syn::parse_str("for i in 0..10{}")?;
//     //     println!("{:#?}", expr.expr);

//     //     // let mut ident_visitor = IdentVisitor::new();
//     //     // ident_visitor.visit_block(&block_user);
//     //     // assert!(ident_visitor.raise_errors().is_err());
//     //     // println!("\nFOund names {:?}", ident_visitor.names);
//     //     // println!("Moved names {:?}\n", ident_visitor.moved_names());
//     //     assert!(false);
//     //     Ok(())
//     // }
// }
