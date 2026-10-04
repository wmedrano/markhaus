//! Editable UI text. Keep these values as plain text; templates handle HTML escaping.

/// Header, footer, and browser title shared by every page.
pub(crate) struct SiteText {
    pub brand: &'static str,
    pub brand_period: &'static str,
    pub brand_home_label: &'static str,
    pub title_brand: &'static str,
    pub header_note: &'static str,
    pub skip_link: &'static str,
    pub footer_text: &'static str,
    pub footer_signature: &'static str,
}

impl Default for SiteText {
    fn default() -> Self {
        Self {
            brand: "markhaus",
            brand_period: ".",
            brand_home_label: "Markhaus home",
            title_brand: "Markhaus",
            header_note: "A SPACE FOR DOCUMENTS",
            skip_link: "Skip to content",
            footer_text: "Simple files. Simple serving.",
            footer_signature: "MARKHAUS",
        }
    }
}

/// Homepage/title screen, ordered from the hero to the collection.
pub(crate) struct HomeText {
    pub eyebrow: &'static str,
    /// Each entry is a line; the last headline line uses the accent color.
    pub headline: [&'static str; 3],
    pub description: [&'static str; 2],
    pub composition_label: &'static str,
    pub composition_caption: &'static str,
    pub collection_title: &'static str,
    pub document_singular: &'static str,
    pub document_plural: &'static str,
    pub file_kind: &'static str,
    pub empty_title: &'static str,
    pub empty_live_message: &'static str,
    pub empty_export_message: &'static str,
}

impl Default for HomeText {
    fn default() -> Self {
        Self {
            eyebrow: "A little order for your ideas",
            headline: ["Words.", "In good", "form."],
            description: [
                "Your readable Markdown.",
                "Pick a document and make yourself at home.",
            ],
            composition_label: "Markdown",
            composition_caption: "THE READING ROOM — № 01",
            collection_title: "The collection",
            document_singular: "document",
            document_plural: "documents",
            file_kind: "Markdown",
            empty_title: "A blank canvas.",
            empty_live_message: "Add a .md or .markdown file to this directory, then refresh to start reading.",
            empty_export_message: "No Markdown documents were found in the source directory.",
        }
    }
}
