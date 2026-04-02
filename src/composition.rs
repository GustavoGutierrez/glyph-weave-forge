use crate::adapters::render::MinimalPdfRenderer;
#[cfg(feature = "renderer-typst")]
use crate::adapters::render::TypstPdfRenderer;
#[cfg(feature = "fs")]
use crate::adapters::resources::FilesystemResolver;
use crate::api::{ConvertOptions, RenderBackendSelection};
use crate::core::ports::{RenderBackend, ResourceResolver};

pub fn ensure_renderer(options: &mut ConvertOptions<'_>) {
    if options.renderer.is_none() {
        options.renderer = Some(default_renderer(options.backend));
    }
}

pub fn default_renderer(selection: RenderBackendSelection) -> Box<dyn RenderBackend> {
    match selection {
        RenderBackendSelection::Minimal => Box::new(MinimalPdfRenderer),
        #[cfg(feature = "renderer-typst")]
        RenderBackendSelection::Typst => Box::new(TypstPdfRenderer),
    }
}

pub fn default_resource_resolver() -> Option<&'static dyn ResourceResolver> {
    #[cfg(feature = "fs")]
    {
        static RESOLVER: FilesystemResolver = FilesystemResolver;
        Some(&RESOLVER)
    }
    #[cfg(not(feature = "fs"))]
    {
        None
    }
}
