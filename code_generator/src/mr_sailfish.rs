use sailfish::runtime::{escape, Buffer, Render, RenderError};
use syn::FieldValue;

// Create a macro to impl each syn type
impl Render for syn::FieldValue {
    #[inline]
    fn render(&self, b: &mut Buffer) -> Result<()> {
        b.push_str(&**self);
        Ok(())
    }

    #[inline]
    fn render_escaped(&self, b: &mut Buffer) -> Result<()> {
        escape::escape_to_buf(&**self, b);
        Ok(())
    }
}
