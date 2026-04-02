use crate::api::{OutputTarget, PdfOutput};
#[cfg(feature = "fs")]
use crate::core::ForgeError;
use crate::core::Result;

pub fn write_output(
    output: &OutputTarget<'_>,
    output_file_name: Option<&str>,
    source_name: &str,
    pdf_bytes: Vec<u8>,
) -> Result<PdfOutput> {
    #[cfg(not(feature = "fs"))]
    let _ = (output_file_name, source_name);

    match output {
        OutputTarget::Memory => Ok(PdfOutput {
            bytes: Some(pdf_bytes),
            #[cfg(feature = "fs")]
            written_path: None,
        }),
        OutputTarget::__Lifetime(_) => unreachable!("lifetime marker is never constructed"),
        #[cfg(feature = "fs")]
        OutputTarget::File(path) => {
            std::fs::write(path, &pdf_bytes).map_err(|source| ForgeError::OutputWrite {
                path: (*path).to_path_buf(),
                source,
            })?;
            Ok(PdfOutput {
                bytes: None,
                written_path: Some((*path).to_path_buf()),
            })
        }
        #[cfg(feature = "fs")]
        OutputTarget::Directory(dir) => {
            std::fs::create_dir_all(dir).map_err(|source| ForgeError::OutputDirectory {
                path: (*dir).to_path_buf(),
                source,
            })?;
            let destination = dir.join(derive_output_name(output_file_name, source_name)?);
            std::fs::write(&destination, &pdf_bytes).map_err(|source| ForgeError::OutputWrite {
                path: destination.clone(),
                source,
            })?;
            Ok(PdfOutput {
                bytes: None,
                written_path: Some(destination),
            })
        }
    }
}

#[cfg(feature = "fs")]
pub fn derive_output_name(output_file_name: Option<&str>, source_name: &str) -> Result<String> {
    let candidate = output_file_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{source_name}.pdf"));

    if candidate.is_empty() {
        return Err(ForgeError::InvalidOutputFileName);
    }

    if candidate.ends_with(".pdf") {
        Ok(candidate)
    } else {
        Ok(format!("{candidate}.pdf"))
    }
}
