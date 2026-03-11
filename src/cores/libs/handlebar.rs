use handlebars::{Context, Handlebars, Helper, HelperResult, Output, RenderContext};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug)]
pub struct Handlebar {
    handlebars: Handlebars<'static>,
}

impl Deref for Handlebar {
    type Target = Handlebars<'static>;

    fn deref(&self) -> &Self::Target {
        &self.handlebars
    }
}

impl DerefMut for Handlebar {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.handlebars
    }
}

fn json_encode(
    h: &Helper,     // Hapus <'static>
    _: &Handlebars, // Hapus <'static>
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> HelperResult {
    let param = h
        .param(0)
        .map(|v| v.value())
        .unwrap_or(&serde_json::Value::Null);

    let html = serde_json::to_string(param).unwrap_or_else(|_| "null".to_string());
    out.write(&html)?;

    Ok(())
}

impl Handlebar {
    pub fn new() -> Self {
        Self {
            handlebars: Self::init_handlebar(),
        }
    }
    fn init_handlebar() -> Handlebars<'static> {
        let mut handlebars = Handlebars::new();
        handlebars.register_helper("json", Box::new(json_encode));
        handlebars
    }
    fn handlebars(&self) -> &Handlebars<'static> {
        &self.handlebars
    }
    fn handlebars_mut(&mut self) -> &mut Handlebars<'static> {
        &mut self.handlebars
    }
}
