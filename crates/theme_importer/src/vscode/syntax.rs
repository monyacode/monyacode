use indexmap::IndexMap;
use serde::Deserialize;
use strum::EnumIter;

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum VsCodeTokenScope {
    One(String),
    Many(Vec<String>),
}

#[derive(Debug, Deserialize)]
pub struct VsCodeTokenColor {
    pub name: Option<String>,
    pub scope: Option<VsCodeTokenScope>,
    pub settings: VsCodeTokenColorSettings,
}

#[derive(Debug, Deserialize)]
pub struct VsCodeTokenColorSettings {
    pub foreground: Option<String>,
    pub background: Option<String>,
    #[serde(rename = "fontStyle")]
    pub font_style: Option<String>,
}

#[derive(Debug, PartialEq, Copy, Clone, EnumIter)]
pub enum MonyaCodeSyntaxToken {
    Attribute,
    Boolean,
    Comment,
    CommentDoc,
    Constant,
    Constructor,
    Embedded,
    Emphasis,
    EmphasisStrong,
    Enum,
    Function,
    Hint,
    Keyword,
    Label,
    LinkText,
    LinkUri,
    Number,
    Operator,
    Predictive,
    Preproc,
    Primary,
    Property,
    Punctuation,
    PunctuationBracket,
    PunctuationDelimiter,
    PunctuationListMarker,
    PunctuationSpecial,
    String,
    StringEscape,
    StringRegex,
    StringSpecial,
    StringSpecialSymbol,
    Tag,
    TextLiteral,
    Title,
    Type,
    Variable,
    VariableSpecial,
    Variant,
}

impl std::fmt::Display for MonyaCodeSyntaxToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MonyaCodeSyntaxToken::Attribute => "attribute",
                MonyaCodeSyntaxToken::Boolean => "boolean",
                MonyaCodeSyntaxToken::Comment => "comment",
                MonyaCodeSyntaxToken::CommentDoc => "comment.doc",
                MonyaCodeSyntaxToken::Constant => "constant",
                MonyaCodeSyntaxToken::Constructor => "constructor",
                MonyaCodeSyntaxToken::Embedded => "embedded",
                MonyaCodeSyntaxToken::Emphasis => "emphasis",
                MonyaCodeSyntaxToken::EmphasisStrong => "emphasis.strong",
                MonyaCodeSyntaxToken::Enum => "enum",
                MonyaCodeSyntaxToken::Function => "function",
                MonyaCodeSyntaxToken::Hint => "hint",
                MonyaCodeSyntaxToken::Keyword => "keyword",
                MonyaCodeSyntaxToken::Label => "label",
                MonyaCodeSyntaxToken::LinkText => "link_text",
                MonyaCodeSyntaxToken::LinkUri => "link_uri",
                MonyaCodeSyntaxToken::Number => "number",
                MonyaCodeSyntaxToken::Operator => "operator",
                MonyaCodeSyntaxToken::Predictive => "predictive",
                MonyaCodeSyntaxToken::Preproc => "preproc",
                MonyaCodeSyntaxToken::Primary => "primary",
                MonyaCodeSyntaxToken::Property => "property",
                MonyaCodeSyntaxToken::Punctuation => "punctuation",
                MonyaCodeSyntaxToken::PunctuationBracket => "punctuation.bracket",
                MonyaCodeSyntaxToken::PunctuationDelimiter => "punctuation.delimiter",
                MonyaCodeSyntaxToken::PunctuationListMarker => "punctuation.list_marker",
                MonyaCodeSyntaxToken::PunctuationSpecial => "punctuation.special",
                MonyaCodeSyntaxToken::String => "string",
                MonyaCodeSyntaxToken::StringEscape => "string.escape",
                MonyaCodeSyntaxToken::StringRegex => "string.regex",
                MonyaCodeSyntaxToken::StringSpecial => "string.special",
                MonyaCodeSyntaxToken::StringSpecialSymbol => "string.special.symbol",
                MonyaCodeSyntaxToken::Tag => "tag",
                MonyaCodeSyntaxToken::TextLiteral => "text.literal",
                MonyaCodeSyntaxToken::Title => "title",
                MonyaCodeSyntaxToken::Type => "type",
                MonyaCodeSyntaxToken::Variable => "variable",
                MonyaCodeSyntaxToken::VariableSpecial => "variable.special",
                MonyaCodeSyntaxToken::Variant => "variant",
            }
        )
    }
}

impl MonyaCodeSyntaxToken {
    pub fn find_best_token_color_match<'a>(
        &self,
        token_colors: &'a [VsCodeTokenColor],
    ) -> Option<&'a VsCodeTokenColor> {
        let mut ranked_matches = IndexMap::new();

        for (ix, token_color) in token_colors.iter().enumerate() {
            if token_color.settings.foreground.is_none() {
                continue;
            }

            let Some(rank) = self.rank_match(token_color) else {
                continue;
            };

            if rank > 0 {
                ranked_matches.insert(ix, rank);
            }
        }

        ranked_matches
            .into_iter()
            .max_by_key(|(_, rank)| *rank)
            .map(|(ix, _)| &token_colors[ix])
    }

    fn rank_match(&self, token_color: &VsCodeTokenColor) -> Option<u32> {
        let candidate_scopes = match token_color.scope.as_ref()? {
            VsCodeTokenScope::One(scope) => vec![scope],
            VsCodeTokenScope::Many(scopes) => scopes.iter().collect(),
        }
        .iter()
        .flat_map(|scope| scope.split(',').map(|s| s.trim()))
        .collect::<Vec<_>>();

        let scopes_to_match = self.to_vscode();
        let number_of_scopes_to_match = scopes_to_match.len();

        let mut matches = 0;

        for (ix, scope) in scopes_to_match.into_iter().enumerate() {
            // Assign each entry a weight that is inversely proportional to its
            // position in the list.
            //
            // Entries towards the front are weighted higher than those towards the end.
            let weight = (number_of_scopes_to_match - ix) as u32;

            if candidate_scopes.contains(&scope) {
                matches += 1 + weight;
            }
        }

        Some(matches)
    }

    pub fn fallbacks(&self) -> &[Self] {
        match self {
            MonyaCodeSyntaxToken::CommentDoc => &[MonyaCodeSyntaxToken::Comment],
            MonyaCodeSyntaxToken::Number => &[MonyaCodeSyntaxToken::Constant],
            MonyaCodeSyntaxToken::VariableSpecial => &[MonyaCodeSyntaxToken::Variable],
            MonyaCodeSyntaxToken::PunctuationBracket
            | MonyaCodeSyntaxToken::PunctuationDelimiter
            | MonyaCodeSyntaxToken::PunctuationListMarker
            | MonyaCodeSyntaxToken::PunctuationSpecial => &[MonyaCodeSyntaxToken::Punctuation],
            MonyaCodeSyntaxToken::StringEscape
            | MonyaCodeSyntaxToken::StringRegex
            | MonyaCodeSyntaxToken::StringSpecial
            | MonyaCodeSyntaxToken::StringSpecialSymbol => &[MonyaCodeSyntaxToken::String],
            _ => &[],
        }
    }

    fn to_vscode(self) -> Vec<&'static str> {
        match self {
            MonyaCodeSyntaxToken::Attribute => vec!["entity.other.attribute-name"],
            MonyaCodeSyntaxToken::Boolean => vec!["constant.language"],
            MonyaCodeSyntaxToken::Comment => vec!["comment"],
            MonyaCodeSyntaxToken::CommentDoc => vec!["comment.block.documentation"],
            MonyaCodeSyntaxToken::Constant => vec!["constant", "constant.language", "constant.character"],
            MonyaCodeSyntaxToken::Constructor => {
                vec!["entity.name.tag", "entity.name.function.definition.special.constructor"]
            }
            MonyaCodeSyntaxToken::Embedded => vec!["meta.embedded"],
            MonyaCodeSyntaxToken::Emphasis => vec!["markup.italic"],
            MonyaCodeSyntaxToken::EmphasisStrong => {
                vec!["markup.bold", "markup.italic markup.bold", "markup.bold markup.italic"]
            }
            MonyaCodeSyntaxToken::Enum => vec!["support.type.enum"],
            MonyaCodeSyntaxToken::Function => vec!["entity.function", "entity.name.function", "variable.function"],
            MonyaCodeSyntaxToken::Hint => vec![],
            MonyaCodeSyntaxToken::Keyword => vec![
                "keyword",
                "keyword.other.fn.rust",
                "keyword.control",
                "keyword.control.fun",
                "keyword.control.class",
                "punctuation.accessor",
                "entity.name.tag",
            ],
            MonyaCodeSyntaxToken::Label => vec!["label", "entity.name", "entity.name.import", "entity.name.package"],
            MonyaCodeSyntaxToken::LinkText => vec!["markup.underline.link", "string.other.link"],
            MonyaCodeSyntaxToken::LinkUri => vec!["markup.underline.link", "string.other.link"],
            MonyaCodeSyntaxToken::Number => vec!["constant.numeric", "number"],
            MonyaCodeSyntaxToken::Operator => vec!["operator", "keyword.operator"],
            MonyaCodeSyntaxToken::Predictive => vec![],
            MonyaCodeSyntaxToken::Preproc => {
                vec!["preproc", "meta.preprocessor", "punctuation.definition.preprocessor"]
            }
            MonyaCodeSyntaxToken::Primary => vec![],
            MonyaCodeSyntaxToken::Property => vec![
                "variable.member",
                "support.type.property-name",
                "variable.object.property",
                "variable.other.field",
            ],
            MonyaCodeSyntaxToken::Punctuation => vec![
                "punctuation",
                "punctuation.section",
                "punctuation.accessor",
                "punctuation.separator",
                "punctuation.definition.tag",
            ],
            MonyaCodeSyntaxToken::PunctuationBracket => vec![
                "punctuation.bracket",
                "punctuation.definition.tag.begin",
                "punctuation.definition.tag.end",
            ],
            MonyaCodeSyntaxToken::PunctuationDelimiter => vec![
                "punctuation.delimiter",
                "punctuation.separator",
                "punctuation.terminator",
            ],
            MonyaCodeSyntaxToken::PunctuationListMarker => {
                vec!["markup.list punctuation.definition.list.begin"]
            }
            MonyaCodeSyntaxToken::PunctuationSpecial => vec!["punctuation.special"],
            MonyaCodeSyntaxToken::String => vec!["string"],
            MonyaCodeSyntaxToken::StringEscape => {
                vec!["string.escape", "constant.character", "constant.other"]
            }
            MonyaCodeSyntaxToken::StringRegex => vec!["string.regex"],
            MonyaCodeSyntaxToken::StringSpecial => vec!["string.special", "constant.other.symbol"],
            MonyaCodeSyntaxToken::StringSpecialSymbol => {
                vec!["string.special.symbol", "constant.other.symbol"]
            }
            MonyaCodeSyntaxToken::Tag => vec!["tag", "entity.name.tag", "meta.tag.sgml"],
            MonyaCodeSyntaxToken::TextLiteral => vec!["text.literal", "string"],
            MonyaCodeSyntaxToken::Title => vec!["title", "entity.name"],
            MonyaCodeSyntaxToken::Type => vec![
                "entity.name.type",
                "entity.name.type.primitive",
                "entity.name.type.numeric",
                "keyword.type",
                "support.type",
                "support.type.primitive",
                "support.class",
            ],
            MonyaCodeSyntaxToken::Variable => vec![
                "variable",
                "variable.language",
                "variable.member",
                "variable.parameter",
                "variable.parameter.function-call",
            ],
            MonyaCodeSyntaxToken::VariableSpecial => vec![
                "variable.special",
                "variable.member",
                "variable.annotation",
                "variable.language",
            ],
            MonyaCodeSyntaxToken::Variant => vec!["variant"],
        }
    }
}
