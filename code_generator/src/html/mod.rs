use proc_macro2::{Delimiter, Span, TokenTree};
use proc_macro2_diagnostics::SpanDiagnosticExt;
use syn::{
    braced, custom_punctuation,
    ext::IdentExt,
    parse::{discouraged::Speculative, Parse, ParseBuffer, ParseStream},
    token::{Colon, Eq},
    Block, Error, Expr, ExprForLoop, Ident, LitStr, Result, Token,
};

mod utils;

custom_punctuation!(OpenTag, <);
custom_punctuation!(CloseTag, >);
custom_punctuation!(OpenClosingTag, </);
custom_punctuation!(SelfCloseVoidTag, />);

const INNER_ERROR: &str = "Inner element can't be parsed.";

// pub struct Node {
//     el: Element,
//     children: Vec<Node>,
// }

// attrs : while not > { attr }
// <el attrs><inner1></inner1><inner2></inner2></el>
//
// Parse Element
// <el do attrs
// while not </ : Parse Element (children)
// check </Ident>
// return Element

fn ident_in_brace(input: ParseStream) -> Result<Ident> {
    println!("ident in brace");
    input.step(|cursor| {
        if let Some((cursor, _, cursor_after_group)) = cursor.group(Delimiter::Brace) {
            if let Some((ident, cursor)) = cursor.ident() {
                if cursor.eof() {
                    return Ok((ident, cursor_after_group));
                }
            }
        }
        Err(Error::new(cursor.span(), "Won't be raised"))
    })
}
fn lit_in_brace(input: ParseStream) -> Result<LitStr> {
    // fork is needed to not mess up Block's parsing
    // another way would be to use input.step
    let fork = input.fork();
    let inner = parse_brace(&fork)?;
    let diagnostic = inner.span().warning(
        "TODO: figure it out a better way to handle spaces \
        (by now you need a block containing a string).",
    );
    let lit: LitStr = inner.parse()?;
    diagnostic.emit_as_expr_tokens();
    if !inner.is_empty() {
        return Err(Error::new(
            inner.span(),
            "This block doesn't contains only a LitStr.",
        ));
    }
    input.advance_to(&fork);
    Ok(lit)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttrExprType {
    String(String),
    Ident(Ident),
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub expr: AttrExprType,
    pub namespace: String,
}

impl Parse for Attribute {
    fn parse(input: ParseStream) -> Result<Self> {
        println!("Parse Attribute !");
        // let namespace_ident = input.call(Ident::parse_any)?;
        let namespace_punct =
            syn::punctuated::Punctuated::<Ident, syn::token::Sub>::parse_separated_nonempty_with(
                input,
                Ident::parse_any,
            )?;
        // Can't panic on first, parse_separated_nonempty return at least one element (or Err)
        let namespace_ident = namespace_punct.first().unwrap();
        let mut namespace: String = quote::quote!(#namespace_ident).to_string();
        println!("NAMESPACE: {namespace}");
        let name: String;
        if input.peek(Colon) {
            let _: Colon = input.parse()?;
            // name = input.call(Ident::parse_any)?.to_string();
            let name_punct =
                syn::punctuated::Punctuated::<Ident, syn::token::Sub>::parse_separated_nonempty_with(
                    input,
                    Ident::parse_any
                )?;
            name = quote::quote!(#name_punct).to_string();
            println!("NAME: {name}");
        } else {
            name = namespace;
            namespace = "".to_string();
        }
        let expr: AttrExprType;
        if input.peek(Token![=]) {
            // <... name=...>
            let _: Eq = input.parse()?;
            if input.peek(LitStr) {
                // <... name="value">
                let value: LitStr = input.parse()?;
                expr = AttrExprType::String(value.value());
            } else if input.peek(Ident) {
                // <... name=value>
                let value: Ident = input.parse()?;
                expr = AttrExprType::String(value.to_string());
            } else {
                println!("Parse block or ident.");
                // <... name={some rust code}>
                if let Ok(ident) = ident_in_brace(input) {
                    // <... name={only_one_ident}>
                    println!("Ident FOUND");
                    expr = AttrExprType::Ident(ident);
                } else {
                    // <... name={longer block...}>
                    // expr = AttrExprType::Block(input.parse()?);
                    println!("Block FOUND");
                    // let value: Block = input.parse()?;
                    let value: Block = input.parse().map_err(|err| {
                        Error::new(err.span(), format!("Error in rust block : {}", err))
                    })?;
                    // Err(Error::new(
                    //     value.brace_token.span,
                    //     "Blocks are not implemented yet (use ident).",
                    // ))?;
                    println!("BLock parsed without err");
                    expr = AttrExprType::Block(value);
                }
            }
        } else {
            // <... name>
            expr = AttrExprType::String("".to_string());
        }

        // Emit a warning for useless namespaces
        if namespace != "" {
            if let AttrExprType::String(_) = expr {
                namespace_ident
                    .span()
                    .warning("W001: Namespace is useless without code's block.")
                    .emit_as_expr_tokens();
            }
        }
        Ok(Attribute {
            name,
            namespace,
            expr,
        })
    }
}

struct ForLoopWithoutBlock {
    expr: ExprForLoop,
}

impl Parse for ForLoopWithoutBlock {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let attrs = input.call(syn::Attribute::parse_outer)?;
        let label: Option<syn::Label> = input.parse()?;
        let for_token: Token![for] = input.parse()?;
        let pat = utils::multi_pat_with_leading_vert(&input)?;

        let in_token: Token![in] = input.parse()?;
        let expr: Expr = input.call(Expr::parse_without_eager_brace)?;

        let for_loop = ExprForLoop {
            attrs,
            label,
            // attrs: Vec::new(),
            // label: None,
            for_token,
            pat,
            in_token,
            expr: Box::new(expr),
            body: syn::parse_str("{}")?,
        };
        println!("Loop created");
        println!("{:#?}", for_loop);
        Ok(ForLoopWithoutBlock { expr: for_loop })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Condition {
    pub expr: Expr,
    pub children: Vec<Element>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExprElement {
    For {
        expr: ExprForLoop,
        children: Vec<Element>,
    },
    If {
        conditions: Vec<Condition>,
    },
    Block(Block),
    Ident(Ident),
    Lit(LitStr),
}

fn parse_brace_fork(input: ParseStream) -> Result<(ParseBuffer, ParseBuffer)> {
    let fork = input.fork();
    let inner;
    braced!(inner in fork);
    Ok((fork, inner))
}

fn parse_brace(input: ParseStream) -> Result<ParseBuffer> {
    let inner;
    braced!(inner in input);
    Ok(inner)
}

// fn handle_inner_children(input: ParseStream) -> Vec<Element> {
// }
fn check_tag_name(name: &Ident, close_name: Ident) -> Result<()> {
    if close_name.to_string() != name.to_string() {
        let mut error = Error::new(
            close_name.span(),
            "Closing element's name does not match an opening element's name.",
        );
        error.combine(Error::new(
            name.span(),
            "No closing element's name match this elements name.",
        ));
        Err(error)
    } else {
        Ok(())
    }
}

fn double_error(ident_1: &Span, msg_1: &str, ident_2: &Span, msg_2: &str) -> Result<Error> {
    let mut error = Error::new(ident_1.clone(), msg_1);
    error.combine(Error::new(ident_2.clone(), msg_2));
    Err(error)
}
struct IfChildrenCtx {
    children: Vec<Element>,
    conditions: Vec<Condition>,
    if_reached: bool,
    else_reached: bool,
    if_span: Span,
    name: Span,
    expr: Expr,
}
impl IfChildrenCtx {
    fn new(fork: ParseBuffer) -> Result<Self> {
        println!("New IfChildren");
        let if_span = fork.parse::<Token![if]>()?.span;
        let name = if_span.clone();
        Ok(Self {
            children: Vec::new(),
            conditions: Vec::new(),
            if_reached: false,
            else_reached: false,
            if_span,
            name,
            expr: fork.call(Expr::parse_without_eager_brace)?,
        })
    }
    fn create_previous_condition(&mut self) {
        self.conditions.push(Condition {
            expr: self.expr.clone(),
            children: self.children.drain(..).collect(),
        });
    }
    fn push(&mut self, child: Element) {
        self.children.push(child);
    }
}

fn parse_expr_without_eager_brace(inner: ParseStream) -> Result<Expr> {
    inner.call(Expr::parse_without_eager_brace).map_err(|_| {
        Error::new(
            inner.span(),
            "An expression is required for `if`, `else if` and `for`.",
        )
    })
}

impl Parse for ExprElement {
    fn parse(input: ParseStream) -> Result<Self> {
        println!("START parse EXPR");
        let fork_input = input.fork();
        let fork_inner = parse_brace(&fork_input)?;

        if fork_inner.peek(Token![for]) {
            let inner = &parse_brace(input)?;
            let mut children: Vec<Element> = Vec::new();
            let open_token: Token![for] = inner.fork().parse()?;
            let expr = inner.parse::<ForLoopWithoutBlock>()?.expr;
            println!("Children -> 'for'");
            // Collect children Elements
            loop {
                match parse_brace_fork(input) {
                    // Braces found
                    Ok((fork_brace, inner_brace)) => match inner_brace.parse::<Token![/]>() {
                        // close tag found
                        Ok(_) => {
                            if let Err(_) = inner_brace.parse::<Token![for]>() {
                                double_error(
                                        &open_token.span,
                                        "Closing element's name does not match an opening element's name.",
                                        &inner_brace.span(),
                                        "No closing element's name match this elements name.",
                                    )?;
                            };
                            input.advance_to(&fork_brace);
                            break;
                        }
                        // inner expr found
                        Err(_) => children.push(Element::ExprElement(input.parse()?)),
                    },
                    // braces not found : inner element found (else than expr)
                    Err(_) => children.push(input.parse()?),
                }
            }
            Ok(ExprElement::For { expr, children })
        } else if fork_inner.peek(Token![if]) {
            let mut ctx = IfChildrenCtx::new(fork_inner)?;
            println!("CTX initialized");

            // Collect children Elements
            // println!("Children -> '{}'", ctx.name.to_string());
            loop {
                match parse_brace(&input.fork()) {
                    Ok(fork) => {
                        if fork.peek(Token![/]) {
                            println!("REACH {{/if}}");
                            let inner = parse_brace(input)?;
                            inner.parse::<Token![/]>()?;
                            if let Err(last) = inner.parse::<Token![if]>() {
                                double_error(
                                    &ctx.if_span,
                                    "Closing element's name does not match an opening element's name.",
                                    &last.span(),
                                    "No closing element's name match this elements name.",
                                )?;
                            };
                            ctx.create_previous_condition();
                            break;
                        } else if fork.peek(Token![if]) {
                            println!("REACH {{if ...}}");
                            let inner = parse_brace(input)?;
                            let token = inner.parse::<Token![if]>()?;
                            println!("parsed brace");
                            if ctx.if_reached {
                                println!("already reached");
                                double_error(
                                    &ctx.if_span,
                                    "Consider to close this if ({/if}) or an else if.",
                                    &token.span,
                                    "If can't follow a not-closed if.",
                                )?;
                            } else {
                                ctx.name = token.span;
                                ctx.if_reached = true;
                                println!("parse without brace :");
                                println!("inner is : {}", inner);
                                ctx.expr = parse_expr_without_eager_brace(&inner)?;
                                println!("parsed !");
                            }
                        } else if fork.peek(Token![else]) && fork.peek2(Token![if]) {
                            println!("REACH {{else if ...}}");
                            let inner = parse_brace(input)?;
                            let token = inner.parse::<Token![else]>()?;
                            // ctx.name = inner.parse()?;
                            if !ctx.if_reached {
                                Err(Error::new(
                                    // ctx.name.span(),
                                    token.span,
                                    "Else if clause must be preceded by an if. 
                                    Consider to convert this else if into a simple if.",
                                ))?
                            } else {
                                ctx.create_previous_condition();
                                ctx.name = token.span;
                                inner.parse::<Token![if]>()?;
                                ctx.expr = parse_expr_without_eager_brace(&inner)?;
                            }
                        } else if fork.peek(Token![else]) {
                            println!("REACH {{else}}");
                            let inner = parse_brace(input)?;
                            let token = inner.parse::<Token![else]>()?;
                            if ctx.else_reached {
                                double_error(
                                    &ctx.name,
                                    "Else clause can't follow another.",
                                    &token.span,
                                    "Consider to remove this else block.",
                                )?;
                            } else if !ctx.if_reached {
                                println!("parse inner else:");
                                Err(Error::new(
                                    token.span,
                                    "Else if clause must be preceded by an if. 
                                    Consider to remove this else (only if).",
                                ))?
                            } else {
                                println!("parse inner else:");
                                ctx.name = token.span;
                                ctx.else_reached = true;
                                ctx.create_previous_condition();
                                ctx.expr = syn::parse_str("true")?;
                            }
                        } else {
                            println!("REACH something else then if clause");
                            double_error(
                                &ctx.if_span,
                                "If clause must be closed ({/if}).",
                                &fork.span(),
                                // &fork.parse()?.span(),
                                "Consider to insert a closing tag before this one ({/if}).",
                            )?;
                        }
                    }
                    Err(_) => {
                        println!("REACH inner element");

                        ctx.push(input.parse()?);
                    }
                }
            }
            Ok(ExprElement::If {
                conditions: ctx.conditions,
            })
        // } else if input.peek(Token![else]) {
        //     if input.peek(Token![if]) {
        //         input.parse::<Token![if]>()?;
        //         element = ExprElement::ElseIf {
        //             expr: input.parse()?,
        //             children,
        //         };
        //         name = "else_if".to_string();
        //     } else {
        //         element = ExprElement::Else { children };
        //         name = "else_if".to_string();
        //     }
        } else {
            println!("Parse other EXPR");
            match ident_in_brace(input) {
                Ok(ident) => Ok(ExprElement::Ident(ident)),
                Err(_) => match lit_in_brace(input) {
                    Ok(lit) => Ok(ExprElement::Lit(lit)),
                    Err(_) => {
                        input
                            .span()
                            .warning("Block Element are in early stages. Don't hesitate to make proposals.")
                            .emit_as_expr_tokens();

                        Ok(ExprElement::Block(input.parse()?))
                    }
                },
            }
        }
    }
}

fn parse_string_until_string(input: ParseStream, target_str: &str) -> Result<String> {
    let targets: Vec<char> = target_str.chars().collect();
    // let extracted_string: String;
    Ok(input.step(|cursor| {
        let mut rest = *cursor;
        let mut stack: Vec<String> = Vec::new();
        let mut current_match = 0;
        while let Some((tt, next)) = rest.token_tree() {
            stack.push(tt.to_string());
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == targets[current_match] => {
                    current_match += 1;
                    if current_match == targets.len() {
                        return Ok((stack.join(""), next));
                    } else {
                        rest = next
                    }
                }
                _ => {
                    current_match = 0;
                    rest = next
                }
            }
        }
        // Err(Error(cursor.span(), "Can't find closing >"))?
        // let mut err = Error::new(
        //     input.span(),
        //     format!("Should be closed with `{}`", target_str),
        // );
        // err.combine(cursor.error(format!("no `{}` was found after this point", target_str)));
        // Err(err)
        Err(cursor.error(format!("no `{}` was found after this point", target_str)))
    })?)
    // Ok(extracted_string)
}

fn parse_text_element(input: ParseStream) -> Result<String> {
    Ok(input.step(|cursor| {
        let mut rest = *cursor;
        let mut stack: Vec<String> = Vec::new();
        while let Some((tt, next)) = rest.token_tree() {
            println!("TT : '{}'", tt);
            match &tt {
                TokenTree::Punct(punct) if punct.as_char() == '<' => {
                    return Ok((stack.join(""), rest));
                }
                TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                    return Ok((stack.join(""), rest));
                }
                _ => {
                    stack.push(tt.to_string());
                    rest = next;
                }
            }
        }
        Ok((stack.join(""), rest))
        // Err(cursor.error(format!("Neither { or < was found after this point.", neither)))
    })?)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Classic {
    pub name: String,
    pub attrs: Vec<Attribute>,
    pub children: Vec<Element>,
}

impl Parse for Classic {
    fn parse(input: ParseStream) -> Result<Self> {
        // Consume opening tokens
        println!("OpenTag");
        let _: OpenTag = input.parse()?; // Should not raise because of previous peek

        // Check that content is closing correctly without consuming (fork)
        println!("Content");
        let _content = parse_string_until_string(&input.fork(), ">")?;
        println!("Parse ident");
        let name: Ident = input
            .parse()
            .map_err(|err| Error::new(err.span(), "Can't parse ident."))?;
        println!("Attrs -> '{}'", name.to_string());

        // Collect Attributes
        let mut attrs = Vec::new();
        while !(input.peek(CloseTag) || input.peek(SelfCloseVoidTag)) {
            if input.peek(CloseTag) {
                break;
            } else if input.peek(SelfCloseVoidTag) {
                return Ok(Self {
                    name: name.to_string(),
                    attrs,
                    children: Vec::new(),
                });
            } else {
                attrs.push(input.parse()?);
            }
        }

        let mut children = Vec::new();
        if input.peek(SelfCloseVoidTag) {
            // TODO: handle Void element without self closing
            // Self Close Void element
            input.parse::<SelfCloseVoidTag>()?;
        } else if input.peek(CloseTag) {
            input.parse::<CloseTag>()?;

            // Collect children Elements
            println!("Children -> '{}'", name.to_string());
            while !input.peek(OpenClosingTag) {
                children.push(input.parse()?)
            }

            // close element
            println!("\nClose tag -> '{}'", name.to_string());
            let _: OpenClosingTag = input.parse().map_err(|err| {
                println!("Can't close {}", name.to_string());
                let mut full_error = Error::new(name.span(), "Element is not closed.");
                full_error.combine(err);
                full_error
            })?;
            println!("Close Ident -> '{}'", name.to_string());
            // let close_name: Ident = input.parse()?;
            // println!(
            //     "Close Assert Ident -> '{}' and '{}'",
            //     name.to_string(),
            //     close_name.to_string()
            // );
            check_tag_name(&name, input.parse()?)?;
            println!("Close tag ended -> '{}'", name.to_string());
            let _: CloseTag = input.parse()?;
        }

        Ok(Self {
            name: name.to_string(),
            attrs,
            children,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Element {
    Classic(Classic),
    ExprElement(ExprElement),
    Comment(String),
    Text(String),
}

impl Parse for Element {
    fn parse(input: ParseStream) -> Result<Self> {
        println!("\nParse Element");
        Ok(if input.peek(Token![<]) {
            if input.peek2(Token![!]) {
                Self::Comment(parse_string_until_string(input, "-->")?)
            } else if input.peek2(Token![/]) {
                Err(Error::new(input.span(), INNER_ERROR))?
            } else {
                println!("Parse Classic");
                Self::Classic(input.parse()?)
            }
        } else {
            println!("Parse else element");
            match parse_text_element(input)?.as_str() {
                // Can't use Token![{] to detect a Brace so we detect it with an empty text
                "" => Self::ExprElement(input.parse()?),
                text => Self::Text(text.to_string()),
            }
        })
    }
}

pub struct Root(pub Vec<Element>);

impl Parse for Root {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut elements: Vec<Element> = Vec::new();
        while !input.is_empty() {
            elements.push(input.parse()?);
        }
        Ok(Root(elements))
    }
}
