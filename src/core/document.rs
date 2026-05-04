use crate::core::ports::ResolvedAsset;

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph {
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    Table {
        alignments: Vec<TableAlignment>,
        headers: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    Quote {
        content: Vec<Inline>,
    },
    Code {
        language: Option<String>,
        code: String,
    },
    Mermaid {
        source: String,
    },
    Math {
        tex: String,
    },
    Image {
        alt: String,
        asset: ResolvedAsset,
    },
    MissingAsset {
        alt: String,
        target: String,
        message: String,
    },
    Unsupported {
        kind: String,
        raw: String,
    },
    ThematicBreak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableAlignment {
    None,
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Code(String),
    Math(String),
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Link { label: Vec<Inline>, target: String },
    Image { alt: String, target: String },
    ResolvedImage { alt: String, asset: ResolvedAsset },
    SoftBreak,
    HardBreak,
}

impl Document {
    pub fn new(blocks: Vec<Block>) -> Self {
        Self { blocks }
    }
}
