use std::io;

use crate::core::ports::{ResolvedAsset, ResourceContext, ResourceResolver};
use crate::core::{ForgeError, Result};

#[cfg(feature = "fs")]
/// Native filesystem resolver for path-based markdown sources.
pub struct FilesystemResolver;

#[cfg(feature = "fs")]
impl ResourceResolver for FilesystemResolver {
    fn resolve(&self, href: &str, ctx: &ResourceContext) -> Result<ResolvedAsset> {
        let Some(base_dir) = ctx.base_dir.as_ref() else {
            return Ok(ResolvedAsset::missing(
                href,
                "filesystem resolution requires a path-based markdown source",
            ));
        };
        let candidate = base_dir.join(href);
        match std::fs::read(&candidate) {
            Ok(bytes) => Ok(ResolvedAsset::loaded(
                href,
                bytes,
                format!("loaded from {}", candidate.display()),
                Some(candidate),
            )),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(ResolvedAsset::missing(
                href,
                format!("resource not found at {}", candidate.display()),
            )),
            Err(err) => Err(ForgeError::Resource {
                target: href.to_owned(),
                message: err.to_string(),
            }),
        }
    }
}

/// Closure-backed resolver for caller-managed asset loading.
pub struct ClosureResolver<F> {
    resolver: F,
}

impl<F> ClosureResolver<F> {
    /// Creates a closure-backed resource resolver.
    pub fn new(resolver: F) -> Self {
        Self { resolver }
    }
}

impl<F> ResourceResolver for ClosureResolver<F>
where
    F: Fn(&str) -> io::Result<Vec<u8>> + Send + Sync,
{
    fn resolve(&self, href: &str, _ctx: &ResourceContext) -> Result<ResolvedAsset> {
        match (self.resolver)(href) {
            Ok(bytes) => {
                #[cfg(feature = "fs")]
                {
                    Ok(ResolvedAsset::loaded(
                        href,
                        bytes,
                        "loaded through custom resource resolver",
                        None,
                    ))
                }
                #[cfg(not(feature = "fs"))]
                {
                    Ok(ResolvedAsset::loaded(
                        href,
                        bytes,
                        "loaded through custom resource resolver",
                    ))
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(ResolvedAsset::missing(
                href,
                "custom resolver could not find resource",
            )),
            Err(err) => Err(ForgeError::Resource {
                target: href.to_owned(),
                message: err.to_string(),
            }),
        }
    }
}
