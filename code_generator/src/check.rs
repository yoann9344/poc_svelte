use syn::Result;

use crate::{
    html::{AttrExprType, Attribute, Classic, Element, ExprElement},
    state_block::LocalDetails,
};

fn check_ident_expr_attrs(
    _element: &Element,
    attrs: &Vec<Attribute>,
    details: &LocalDetails,
) -> Result<()> {
    for attr in attrs {
        match attr.expr {
            AttrExprType::Ident(ref ident) => {
                if attr.namespace == "on" {
                    details.events_contains_ident(ident)?;
                } else {
                    details.states_contains_ident(ident)?;
                }
            }
            _ => (),
        }
    }
    Ok(())
}

pub fn check_ident_expr(elements: &Vec<Element>, details: &LocalDetails) -> Result<()> {
    for el in elements {
        match el {
            Element::Classic(Classic {
                ref attrs,
                ref children,
                ..
            }) => {
                check_ident_expr_attrs(&el, attrs, details)?;
                check_ident_expr(children, details)?;
            }
            Element::ExprElement(el_expr) => match el_expr {
                ExprElement::Ident(ref ident) => details.states_contains_ident(ident)?,
                _ => (),
            },
            _ => {}
        }
    }
    Ok(())
}
