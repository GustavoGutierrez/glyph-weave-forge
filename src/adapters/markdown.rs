use pulldown_cmark::{CodeBlockKind, Event, HeadingLevel, Options, Parser, Tag, TagEnd};

use crate::core::ports::MarkdownParser;
use crate::core::{Block, Document, Inline, Result};

#[derive(Debug, Default)]
pub struct PulldownParser;

impl MarkdownParser for PulldownParser {
    fn parse(&self, markdown: &str) -> Result<Document> {
        Ok(parse_document(markdown))
    }
}

#[derive(Debug)]
enum InlineFrame {
    Emphasis(Vec<Inline>),
    Strong(Vec<Inline>),
    Link { target: String, label: Vec<Inline> },
    Image { target: String, alt: String },
}

#[derive(Debug)]
struct InlineState {
    root: Vec<Inline>,
    stack: Vec<InlineFrame>,
}

impl InlineState {
    fn new() -> Self {
        Self {
            root: Vec::new(),
            stack: Vec::new(),
        }
    }

    fn push(&mut self, inline: Inline) {
        match self.stack.last_mut() {
            Some(InlineFrame::Emphasis(content)) | Some(InlineFrame::Strong(content)) => {
                content.push(inline)
            }
            Some(InlineFrame::Link { label, .. }) => label.push(inline),
            Some(InlineFrame::Image { alt, .. }) => alt.push_str(&inline_to_plain_text(&inline)),
            None => self.root.push(inline),
        }
    }

    fn finish(self) -> Vec<Inline> {
        self.root
    }
}

#[derive(Debug)]
struct ListState {
    ordered: bool,
    items: Vec<Vec<Inline>>,
    current_item: Vec<Inline>,
}

impl ListState {
    fn new(ordered: bool) -> Self {
        Self {
            ordered,
            items: Vec::new(),
            current_item: Vec::new(),
        }
    }
}

#[derive(Debug)]
struct UnsupportedCapture {
    kind: &'static str,
    depth: usize,
    raw: String,
}

fn parse_document(markdown: &str) -> Document {
    let mut blocks = Vec::new();
    let mut inline_state: Option<InlineState> = None;
    let mut heading_level: Option<u8> = None;
    let mut list_state: Option<ListState> = None;
    let mut quote_depth = 0usize;
    let mut unsupported: Option<UnsupportedCapture> = None;
    let mut code_block: Option<(Option<String>, String)> = None;

    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;

    for event in Parser::new_ext(markdown, options) {
        if let Some(capture) = unsupported.as_mut() {
            match &event {
                Event::Start(Tag::Table(_)) | Event::Start(Tag::FootnoteDefinition(_)) => {
                    capture.depth += 1;
                }
                Event::End(TagEnd::Table) | Event::End(TagEnd::FootnoteDefinition) => {
                    if capture.depth == 0 {
                        blocks.push(Block::Unsupported {
                            kind: capture.kind.to_owned(),
                            raw: capture.raw.trim().to_owned(),
                        });
                        unsupported = None;
                        continue;
                    }
                    capture.depth -= 1;
                }
                _ => capture.raw.push_str(event_to_text(&event)),
            }
            continue;
        }

        if let Some((_, code)) = code_block.as_mut() {
            match event {
                Event::Text(text)
                | Event::Code(text)
                | Event::Html(text)
                | Event::InlineHtml(text) => code.push_str(text.as_ref()),
                Event::SoftBreak | Event::HardBreak => code.push('\n'),
                Event::End(TagEnd::CodeBlock) => {
                    if let Some((language, code)) = code_block.take() {
                        blocks.push(Block::Code { language, code });
                    }
                }
                _ => {}
            }
            continue;
        }

        match event {
            Event::Start(Tag::Paragraph) => {
                if list_state.is_none() || inline_state.is_none() {
                    inline_state = Some(InlineState::new());
                }
            }
            Event::End(TagEnd::Paragraph) => {
                if let Some(state) = inline_state.take() {
                    let content = state.finish();
                    if let Some(list) = list_state.as_mut() {
                        append_inline_run(&mut list.current_item, content);
                    } else if quote_depth > 0 {
                        blocks.push(Block::Quote { content });
                    } else {
                        blocks.push(Block::Paragraph { content });
                    }
                }
            }
            Event::Start(Tag::Heading { level, .. }) => {
                heading_level = Some(map_heading_level(level));
                inline_state = Some(InlineState::new());
            }
            Event::End(TagEnd::Heading(_)) => {
                if let (Some(level), Some(state)) = (heading_level.take(), inline_state.take()) {
                    blocks.push(Block::Heading {
                        level,
                        content: state.finish(),
                    });
                }
            }
            Event::Start(Tag::BlockQuote(_)) => {
                quote_depth += 1;
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                quote_depth = quote_depth.saturating_sub(1);
            }
            Event::Start(Tag::List(start)) => {
                list_state = Some(ListState::new(start.is_some()));
            }
            Event::Start(Tag::Item) => {
                inline_state = Some(InlineState::new());
            }
            Event::End(TagEnd::Item) => {
                if let Some(state) = inline_state.take() {
                    let item = state.finish();
                    if let Some(list) = list_state.as_mut() {
                        append_inline_run(&mut list.current_item, item);
                        list.items.push(std::mem::take(&mut list.current_item));
                    }
                }
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(list) = list_state.take() {
                    blocks.push(Block::List {
                        ordered: list.ordered,
                        items: list.items,
                    });
                }
            }
            Event::Start(Tag::Emphasis) => {
                push_frame(&mut inline_state, InlineFrame::Emphasis(Vec::new()))
            }
            Event::End(TagEnd::Emphasis) => pop_frame(&mut inline_state),
            Event::Start(Tag::Strong) => {
                push_frame(&mut inline_state, InlineFrame::Strong(Vec::new()))
            }
            Event::End(TagEnd::Strong) => pop_frame(&mut inline_state),
            Event::Start(Tag::Link { dest_url, .. }) => push_frame(
                &mut inline_state,
                InlineFrame::Link {
                    target: dest_url.to_string(),
                    label: Vec::new(),
                },
            ),
            Event::End(TagEnd::Link) => pop_frame(&mut inline_state),
            Event::Start(Tag::Image { dest_url, .. }) => push_frame(
                &mut inline_state,
                InlineFrame::Image {
                    target: dest_url.to_string(),
                    alt: String::new(),
                },
            ),
            Event::End(TagEnd::Image) => pop_frame(&mut inline_state),
            Event::Start(Tag::CodeBlock(kind)) => {
                let language = match kind {
                    CodeBlockKind::Fenced(info) => info
                        .split_whitespace()
                        .next()
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned),
                    CodeBlockKind::Indented => None,
                };
                code_block = Some((language, String::new()));
            }
            Event::Rule => blocks.push(Block::ThematicBreak),
            Event::TaskListMarker(checked) => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::Text(if checked {
                        "[x] ".to_owned()
                    } else {
                        "[ ] ".to_owned()
                    }));
                }
            }
            Event::Text(text) | Event::Html(text) | Event::InlineHtml(text) => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::Text(text.to_string()));
                }
            }
            Event::Code(code) => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::Code(code.to_string()));
                }
            }
            Event::SoftBreak => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::SoftBreak);
                }
            }
            Event::HardBreak => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::HardBreak);
                }
            }
            Event::InlineMath(text) | Event::DisplayMath(text) => {
                if let Some(state) = inline_state.as_mut() {
                    state.push(Inline::Code(text.to_string()));
                }
            }
            Event::Start(Tag::Table(_)) => {
                unsupported = Some(UnsupportedCapture {
                    kind: "table",
                    depth: 0,
                    raw: String::new(),
                });
            }
            Event::Start(Tag::FootnoteDefinition(_)) => {
                unsupported = Some(UnsupportedCapture {
                    kind: "footnote",
                    depth: 0,
                    raw: String::new(),
                });
            }
            _ => {}
        }
    }

    Document::new(blocks)
}

fn append_inline_run(target: &mut Vec<Inline>, mut content: Vec<Inline>) {
    if !target.is_empty() && !content.is_empty() {
        target.push(Inline::Text(" ".to_owned()));
    }
    target.append(&mut content);
}

fn push_frame(state: &mut Option<InlineState>, frame: InlineFrame) {
    state.get_or_insert_with(InlineState::new).stack.push(frame);
}

fn pop_frame(state: &mut Option<InlineState>) {
    let Some(state) = state.as_mut() else {
        return;
    };
    let Some(frame) = state.stack.pop() else {
        return;
    };
    let inline = match frame {
        InlineFrame::Emphasis(content) => Inline::Emphasis(content),
        InlineFrame::Strong(content) => Inline::Strong(content),
        InlineFrame::Link { target, label } => Inline::Link { label, target },
        InlineFrame::Image { target, alt } => Inline::Image { alt, target },
    };
    state.push(inline);
}

fn inline_to_plain_text(inline: &Inline) -> String {
    match inline {
        Inline::Text(text) | Inline::Code(text) => text.clone(),
        Inline::Emphasis(children) | Inline::Strong(children) => children
            .iter()
            .map(inline_to_plain_text)
            .collect::<Vec<_>>()
            .join(""),
        Inline::Link { label, .. } => label.iter().map(inline_to_plain_text).collect(),
        Inline::Image { alt, .. } | Inline::ResolvedImage { alt, .. } => alt.clone(),
        Inline::SoftBreak | Inline::HardBreak => " ".to_owned(),
    }
}

fn event_to_text<'a>(event: &'a Event<'a>) -> &'a str {
    match event {
        Event::Text(text)
        | Event::Code(text)
        | Event::Html(text)
        | Event::InlineHtml(text)
        | Event::InlineMath(text)
        | Event::DisplayMath(text) => text.as_ref(),
        Event::SoftBreak | Event::HardBreak => "\n",
        Event::TaskListMarker(true) => "[x] ",
        Event::TaskListMarker(false) => "[ ] ",
        _ => "",
    }
}

fn map_heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}
