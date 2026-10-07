use std::collections::HashMap;

use markdown_parser::html_parser::WARP_EMBED_ATTRIBUTE_NAME;
use serde_yaml::Mapping;
use warp_editor::content::markdown::MarkdownStyle;
use warp_editor::render::BLOCK_FOOTER_HEIGHT;
use warp_editor::render::layout::TextLayout;
use warp_editor::render::model::{
    BlockSpacing, BrokenBlockEmbedding, EmbeddedItem, EmbeddedItemHTMLRepresentation,
    EmbeddedItemRichFormat, LaidOutEmbeddedItem,
};
use warpui::AppContext;
use warpui::elements::{Margin, Padding};

/// Spacing of the card a resolved embed would render as; the broken placeholder keeps its width.
const EMBED_CARD_SPACING: BlockSpacing = BlockSpacing {
    margin: Margin::uniform(0.)
        .with_top(8.)
        .with_left(4.)
        .with_bottom(8.)
        .with_right(16.),
    padding: Padding::uniform(8.)
        .with_left(40.)
        .with_top(16.)
        .with_bottom(BLOCK_FOOTER_HEIGHT),
};

/// An embedded Warp Drive object. Drive objects are not available locally, so the embed always
/// renders as broken, but its ID is kept so the Markdown round-trips unchanged.
#[derive(Debug)]
pub struct UnresolvedEmbed {
    hashed_id: String,
}

impl UnresolvedEmbed {
    pub fn new(hashed_id: String) -> Self {
        Self { hashed_id }
    }
}

impl EmbeddedItem for UnresolvedEmbed {
    fn layout(&self, text_layout: &TextLayout, _app: &AppContext) -> Box<dyn LaidOutEmbeddedItem> {
        let width = text_layout.max_width() - EMBED_CARD_SPACING.x_axis_offset();
        Box::new(BrokenBlockEmbedding::new(
            width,
            text_layout.rich_text_styles().base_text.font_size,
        ))
    }

    fn hashed_id(&self) -> &str {
        self.hashed_id.as_str()
    }

    fn to_mapping(&self, _style: MarkdownStyle) -> Mapping {
        let mut mapping = Mapping::default();
        mapping.insert("id".into(), self.hashed_id().into());
        mapping
    }

    fn to_rich_format(&self, _app: &AppContext) -> EmbeddedItemRichFormat<'_> {
        EmbeddedItemRichFormat {
            plain_text: String::new(),
            html: EmbeddedItemHTMLRepresentation {
                element_name: "pre",
                content: String::new(),
                attributes: HashMap::from([(WARP_EMBED_ATTRIBUTE_NAME, self.hashed_id())]),
            },
        }
    }
}
