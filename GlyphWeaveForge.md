# GlyphWeaveForge — Especificación técnica del crate Rust nativo para Markdown → PDF

## Objetivo

`GlyphWeaveForge` es un crate Rust nativo, multiplataforma y preparado para futura integración con WebAssembly, orientado a convertir Markdown en PDF sin depender de Chrome, Puppeteer, wkhtmltopdf ni otros motores externos.[web:50][web:59][web:60]

El diseño prioriza rendimiento, bajo peso operativo, extensibilidad y el uso estricto de estándares Markdown, facilitando su integración como librería embebible dentro de otros crates o aplicaciones Rust.[web:50][web:55][web:60]

## Nombre del crate

Se descarta el uso simple de "Forge" aislado o como sufijo genérico; se propone **GlyphWeaveForge**, un nombre más distintivo e impactante, que contiene la palabra clave y no mostró colisión aparente en los registros actuales de crates, a diferencia de otras variantes evaluadas (como `typeforge` o `md-forge`).[web:63][web:64][web:66][web:67][web:70][web:73][web:76]

## Decisiones críticas y apego al estándar

La arquitectura se apoya en los siguientes principios:

- **Estricto estándar Markdown**: No se inventarán sintaxis no estándar (como bloques `chart` personalizados). El crate procesará únicamente sintaxis oficial y convenciones altamente adoptadas.[web:78][web:79]
- **Imágenes locales y externas**: Soporte completo para renderizar imágenes y vectores (SVG, PNG, JPEG) referenciados desde Markdown, con resolución inteligente de rutas relativas o provisión en memoria.[web:104][web:107]
- **Código y resaltado de sintaxis**: Fiel renderizado de código inline y bloques de código (fenced code blocks) con resaltado nativo según el lenguaje (Rust, Python, JS, etc.) usando el motor tipográfico sin dependencias extra.[web:100][web:101][web:102]
- **Diagramas estándar (Mermaid)**: Se adopta el ecosistema `mermaid` a través de bloques de código (ampliamente extendido en el ecosistema Markdown técnico) para resolver cronogramas y diagramas.[web:82][web:86][web:97]
- **Fórmulas matemáticas**: Manejo mediante `katex-rs` o traducción a fórmulas del motor tipográfico de las sintaxis estándar de Markdown (ej. `$$...$$` y `\(...\)`).[web:49][web:59][web:60]
- **Typst como núcleo tipográfico**: El layout y la generación del PDF recaen en Typst (`typst-as-lib`), que integra natively el resaltado de sintaxis y la inserción de SVGs.[web:50][web:104]

## Librerías recomendadas

| Propósito | Librería principal | Rol recomendado |
|---|---|---|
| Markdown parser | `pulldown-cmark` / `comrak` | Parseo CommonMark/GFM y construcción del modelo intermedio.[web:51] |
| Mermaid headless | `merman` / `rusty_mermaid_svg` | Parseo y render de diagramas Mermaid a SVG sin JavaScript.[web:48][web:58] |
| Fórmulas | `katex-rs` | Render matemático con soporte nativo y WASM.[web:49][web:59] |
| Tipografía, resaltado de código e imágenes | `typst`, `typst-as-lib`, `typst-bake` | Maquetación, resaltado de sintaxis nativo (`#raw`), inserción de imágenes (`#image`) y compilación final a PDF.[web:50][web:55][web:100][web:104] |

## Requisitos funcionales

El crate debe soportar de forma fluida:

1. **Vías de entrada**:
   - Path a archivo Markdown.
   - Texto Markdown en crudo (`&str`, `String`).
   - Bytes Markdown (`&[u8]`, `Vec<u8>`).
2. **Vías de salida**:
   - Archivo PDF en path explícito o en carpeta con nombre derivado.
   - Generación en memoria como binario PDF (`Vec<u8>`), crucial para WebAssembly y APIs.[web:49][web:59]
3. **Temas visuales**:
   - Predefinidos: invoice, scientific article, professional, engineering, informational.
   - Extensibles mediante JSON de configuración.
   - Soporte de PDF multipágina (A4, Letter) o modo "Una sola página" (SinglePage).
4. **Fidelidad al Markdown**:
   - Bloques de código con resaltado de sintaxis correcto basado en la directiva de lenguaje.[web:100][web:102]
   - Imágenes raster y vectoriales (SVG) embebidas correctamente en el layout del PDF.[web:104][web:107]
   - Fórmulas matemáticas estándar KaTeX/LaTeX.[web:49]
   - Diagramas Mermaid usando la directiva ````mermaid`.[web:82][web:97]

## Arquitectura y modelo de diseño

Pipeline propuesto:

1. **Input layer**: Recibe y normaliza `path`, `text` o `bytes`.
2. **Markdown layer**: Convierte a AST usando `pulldown-cmark`.[web:51]
3. **Resource Resolver layer (Nuevo)**: Intercepta nodos de imágenes (`![alt](ruta)`), resuelve las rutas relativas basándose en el contexto del archivo origen, y precarga los SVGs o imágenes rasterizadas necesarias.[web:105][web:108]
4. **Enrichment layer**: Resuelve los bloques interactivos o complejos (ej. transpilar bloques `mermaid` a nodos SVG mediante `rusty_mermaid_svg`).[web:48]
5. **Theme layer**: Construye el entorno Typst aplicando la plantilla base y overrides JSON.
6. **Layout layer**: Genera la sintaxis Typst intermedia; mapea los bloques de código Markdown a directivas `#raw()` para el resaltado automático.[web:100][web:101]
7. **PDF layer**: Llama a `typst-as-lib` para crear el binario PDF.[web:50]

### API pública

```rust
pub enum MarkdownSource<'a> {
    Path(&'a std::path::Path),
    Text(&'a str),
    Bytes(&'a [u8]),
}

pub enum OutputTarget<'a> {
    File(&'a std::path::Path),
    Directory(&'a std::path::Path),
    Memory,
}

pub enum LayoutMode {
    Paged,
    SinglePage,
}

pub enum PageSize {
    A4,
    Letter,
    Legal,
    Custom { width_mm: f32, height_mm: f32 },
}

pub enum BuiltInTheme {
    Invoice,
    ScientificArticle,
    Professional,
    Engineering,
    Informational,
}

pub struct ThemeConfig {
    pub built_in: Option<BuiltInTheme>,
    pub custom_theme_json: Option<serde_json::Value>,
}

pub struct ConvertOptions<'a> {
    pub source: MarkdownSource<'a>,
    pub output: OutputTarget<'a>,
    pub output_file_name: Option<&'a str>,
    pub page_size: PageSize,
    pub layout_mode: LayoutMode,
    pub theme: ThemeConfig,
    // Permite inyectar un closure para resolver rutas relativas en modos in-memory
    pub resource_resolver: Option<Box<dyn Fn(&str) -> Result<Vec<u8>, std::io::Error>>>, 
}

pub struct PdfOutput {
    pub bytes: Option<Vec<u8>>,
    pub written_path: Option<std::path::PathBuf>,
}
```

### Reglas de resolución de recursos e imágenes

Si un Markdown incluye `![Diagrama](./assets/diagrama.svg)`:

- **Si la entrada es por Path**: El crate usa la ruta base del Markdown de origen para ubicar `./assets/diagrama.svg`, leer sus bytes y pasárselos a Typst.[web:104][web:105][web:108]
- **Si la entrada es Text/Bytes**: Como no hay contexto local garantizado, el crate depende del hook opcional `resource_resolver` provisto por el usuario. Si no se provee o la imagen no existe, inserta una caja con el texto "alt" indicando recurso no encontrado (graceful fallback).

Typst soporta SVG, PNG, JPEG y GIF de forma nativa mediante la función `#image(bytes(...), format: "svg")`, eliminando la necesidad de motores externos de rasterización de imágenes.[web:104][web:107]

### Resaltado de código y bloques de sintaxis

Cuando el parser de Markdown detecta un bloque de código:

```markdown
    ```rust
    fn main() { println!("Hola"); }
    ```
```

El crate debe transcribir este nodo a la función nativa de Typst correspondiente:

```typst
#raw(block: true, lang: "rust", "fn main() { println!(\"Hola\"); }")
```

Typst incluye un motor nativo potente que soporta docenas de lenguajes populares, y permite personalizar el tema de colores de resaltado dentro de las plantillas Typst del crate.[web:99][web:100][web:102][web:103]
El código inline se mapea a `#raw("código")`, manteniendo el diseño tipográfico.

## Ejemplo de uso y API ergonómica

```rust
use glyphweaveforge::{Forge, BuiltInTheme, LayoutMode, PageSize};

// Ejemplo 1: Entrada y salida por sistema de archivos
let result = Forge::new()
    .from_path("./docs/spec.md")
    .to_directory("./output")
    .with_theme(BuiltInTheme::Engineering)
    .with_page_size(PageSize::A4)
    .with_layout_mode(LayoutMode::Paged)
    .convert()?;

// Ejemplo 2: Texto a Memoria (WASM / HTTP friendly) con resolución de imágenes
let markdown_text = "![Logo](/img/logo.svg)\n\n```python\nprint('Hola')\n```";

let result_mem = Forge::new()
    .from_text(markdown_text)
    .to_memory()
    .with_resource_resolver(|path| {
        // Lógica custom (ej. hacer fetch a un CDN, o leer del virtual file system en WASM)
        std::fs::read(format!("./virtual_root{}", path))
    })
    .convert()?;

let pdf_bytes = result_mem.bytes.unwrap();
```

## Estructura de Módulos sugerida

```text
src/
  lib.rs
  api/          # Fachada, Builder, y Enums públicos.
  core/         # Tipos y Error handling.
  markdown/     # Parser AST (pulldown-cmark).
  resources/    # Lógica para resolver y cachear rutas e imágenes (SVG/PNG/JPG).
  mermaid/      # Integración Rust nativa para render SVG de Mermaid.
  math/         # Lógica LaTeX / KaTeX-rs.
  theme/        # Sistema de inyección de JSON a plantillas base.
  layout/       # Compilador a AST / Código Typst (Manejo de `#raw` e `#image`).
  pdf/          # Interface con el motor Typst para generar el binario.
```

## Recomendación final

Al no inventar sintaxis propietaria para gráficas y apoyarse estrictamente en Markdown estándar (texto, código, e imágenes) más el subestándar técnico `mermaid`, `GlyphWeaveForge` se mantiene compatible con cualquier editor y linter existente.[web:78][web:79]

El uso inteligente del ecosistema Rust —`pulldown-cmark` para parseo, `merman` para diagramas y **Typst** para maquetación, inserción nativa de SVG y **resaltado de sintaxis de bloques de código**— consolida a `GlyphWeaveForge` como un crate ultraligero, altamente rendidor y fácil de portar a entornos distribuidos como WebAssembly.[web:48][web:51][web:58][web:60][web:100][web:104]
