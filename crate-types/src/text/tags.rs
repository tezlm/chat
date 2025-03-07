use url::Url;

use super::{Tag, Text};
use crate::{RoleId, RoomId, ThreadId, UserId};

#[cfg(feature = "formatting_extra")]
use crate::{util::Time, EmojiId};

// TODO: stronger typing
// some of these could have less cloning
// some of these are planned to be implemented
/// currently supported tags
#[cfg(not(feature = "formatting_extra"))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KnownTag<'a> {
    /// bold text
    Bold(Text<'a>),

    /// emphasized
    Emphasis(Text<'a>),

    /// strikethrough
    Strikethrough(Text<'a>),

    /// link (optional custom text)
    Link(Url, Option<Text<'a>>),

    /// inline code (optional programming language)
    Code(Text<'a>, Option<String>),

    Mention(MentionTag),

    #[cfg(feature = "formatting_extra")]
    /// subscript (may be removed?)
    Sub(Text<'a>),

    #[cfg(feature = "formatting_extra")]
    /// superscript (may be removed?)
    Sup(Text<'a>),

    #[cfg(feature = "formatting_extra")]
    /// spoiler (optional reason)
    // TODO: accepted, needs impl
    Spoiler(Text<'a>, Option<String>),

    #[cfg(feature = "formatting_extra")]
    /// keyboard shortcut
    // TODO: accepted, needs impl
    Keyboard(Text<'a>),

    #[cfg(feature = "formatting_extra")]
    /// abbreviation
    // TODO: accepted, needs impl
    Abbr(Text<'a>, Text<'a>),

    #[cfg(feature = "formatting_extra")]
    // math/latex (how do i standardize this?)
    Math(&'a str),

    #[cfg(feature = "formatting_extra")]
    /// custom emoji
    Emoji(EmojiId),

    #[cfg(feature = "formatting_extra")]
    /// timestamp
    // TODO: accepted, needs impl
    Time(Time, TimeFormat),
    // Measurement(Unit<'a>),
    // Silence(Text<'a>), // suppress mentions, link embeds (or make them separate)
    // Document(DocumentTag<'a>),
    // Interactive(InteractiveTag<'a>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MentionTag {
    /// mention a user
    MentionUser(UserId),

    /// mention/link a room
    MentionRoom(RoomId),

    /// mention/link a thread
    MentionThread(ThreadId),

    /// mention everyone with a role
    MentionRole(RoleId),

    /// mention everyone in the room
    MentionAllRoom,

    /// mention everyone in the thread
    MentionAllThread,
}

/// how the time should be displayed
// also might be useful to have a duration format?
#[cfg(feature = "formatting_extra")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeFormat {
    TimeShort,
    TimeLong,
    DateShort,
    DateLong,
    DateTimeShort,
    DateTimeLong,
    Relative,
}

// /// what unit this is, automatically localized if necessary
// // do i let people send units losslessly (original unit)
// // honestly, this is probably excessive overkill
// // #[cfg(feature = "formatting_extra")]
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum Unit {
//     Duration { seconds: f64 },
//     Length { meters: f64 },
//     Mass { kilograms: f64 },
//     Current { ampere: f64 },
//     Temperature { kelvin: f64 },
//     Matter { mole: f64 },
//     Lumosity { candela: f64 },
//     Color { color: Color },
//     Angle { rad: f64 },
//     Speed { meters_per_second: f64 },
//     Area { square_meters: f64 },
//     Volume { cube_meters: f64 },
//     Custom {
//         name: String,
//         /// suffix
//         short: String,
//     },
// }

impl<'a> TryFrom<Tag<'a>> for KnownTag<'a> {
    type Error = ();

    fn try_from(value: Tag<'a>) -> Result<Self, Self::Error> {
        match (&*value.name, value.params.as_slice()) {
            ("b", [t]) => Ok(KnownTag::Bold(t.clone())),
            ("em", [t]) => Ok(KnownTag::Emphasis(t.clone())),
            ("a", [l]) => Ok(KnownTag::Link(
                l.as_plain().to_string().parse().map_err(|_| ())?,
                None,
            )),
            ("a", [l, t]) => Ok(KnownTag::Link(
                l.as_plain().to_string().parse().map_err(|_| ())?,
                Some(t.clone()),
            )),
            #[cfg(feature = "formatting_extra")]
            ("sub", [t]) => Ok(KnownTag::Sub(t.clone())),
            #[cfg(feature = "formatting_extra")]
            ("sup", [t]) => Ok(KnownTag::Sup(t.clone())),
            ("s", [t]) => Ok(KnownTag::Strikethrough(t.clone())),
            ("code", [t]) => Ok(KnownTag::Code(t.clone(), None)),
            ("code", [t, l]) => Ok(KnownTag::Code(t.clone(), Some(l.as_plain().to_string()))),
            _ => Err(()),
        }
    }
}

// TODO: test serde, use less copypasta
#[cfg(not(feature = "formatting_extra"))]
mod doc {
    use serde::{Deserialize, Serialize};

    use crate::text::{Language, OwnedText, Text};

    /// text with block level formatting
    /// mainly sticking to markdown-esque formatting for now
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Document(Vec<Block>);

    /// a single unit of block level formatting
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Block {
        Paragraph {
            text: OwnedText,
        },

        Heading {
            text: OwnedText,
            /// between 1 to 6 (same as html/markdown)
            level: u8,
        },

        Blockquote {
            text: Document,
        },

        Code {
            /// if None, try to guess lang (Language("Plain") to explicitly prevent syntax hl)
            lang: Option<Language>,
            text: OwnedText,
        },

        ListUnordered {
            items: Vec<Document>,
        },

        ListOrdered {
            items: Vec<Document>,
        },
    }

    #[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
    #[serde(tag = "type")]
    enum BlockSerde {
        Paragraph {
            text: OwnedText,
        },

        Heading {
            text: OwnedText,
            /// between 1 to 6 (same as html/markdown)
            level: u8,
        },

        Blockquote {
            text: Document,
        },

        Code {
            /// if None, try to guess lang (Language("Plain") to explicitly prevent syntax hl)
            lang: Option<Language>,
            text: OwnedText,
        },

        ListUnordered {
            items: Vec<Document>,
        },

        ListOrdered {
            items: Vec<Document>,
        },
    }

    impl From<Block> for BlockSerde {
        fn from(value: Block) -> Self {
            match value {
                Block::Paragraph { text } => BlockSerde::Paragraph { text },
                Block::Heading { text, level } => BlockSerde::Heading { text, level },
                Block::Blockquote { text } => BlockSerde::Blockquote { text },
                Block::Code { lang, text } => BlockSerde::Code { lang, text },
                Block::ListUnordered { items } => BlockSerde::ListUnordered { items },
                Block::ListOrdered { items } => BlockSerde::ListOrdered { items },
            }
        }
    }

    impl From<BlockSerde> for Block {
        fn from(value: BlockSerde) -> Self {
            match value {
                BlockSerde::Paragraph { text } => Block::Paragraph { text },
                BlockSerde::Heading { text, level } => Block::Heading { text, level },
                BlockSerde::Blockquote { text } => Block::Blockquote { text },
                BlockSerde::Code { lang, text } => Block::Code { lang, text },
                BlockSerde::ListUnordered { items } => Block::ListUnordered { items },
                BlockSerde::ListOrdered { items } => Block::ListOrdered { items },
            }
        }
    }

    impl Serialize for Document {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if self.0.len() == 1 {
                self.0.serialize(serializer)
            } else {
                serializer.collect_seq(&self.0)
            }
        }
    }

    impl Serialize for Block {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            if let Block::Paragraph { text } = self {
                text.serialize(serializer)
            } else {
                let a: BlockSerde = self.clone().into();
                a.serialize(serializer)
            }
        }
    }

    impl<'de> Deserialize<'de> for Document {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum AnyDocument {
                Many(Vec<Block>),
                One(Block),
            }

            match AnyDocument::deserialize(deserializer)? {
                AnyDocument::Many(v) => Ok(Self(v)),
                AnyDocument::One(o) => Ok(Self(vec![o])),
            }
        }
    }

    impl<'de> Deserialize<'de> for Block {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum AnyBlock {
                Str(String),
                Obj(BlockSerde),
            }

            match AnyBlock::deserialize(deserializer)? {
                AnyBlock::Str(s) => Ok(Block::Paragraph {
                    text: Text::parse(&s).to_owned(),
                }),
                AnyBlock::Obj(o) => Ok(o.into()),
            }
        }
    }
}

#[cfg(feature = "formatting_extra")]
mod doc {
    use crate::{misc::Color, text::Text, Media};

    pub struct Document<'a>(Vec<Block<'a>>);

    /// block level formatting (WIP)
    // #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Block<'a> {
        /// inline text, can be a plain string
        Text(Text<'a>),

        H1(Text<'a>),
        H2(Text<'a>),
        H3(Text<'a>),
        H4(Text<'a>),
        H5(Text<'a>),
        H6(Text<'a>),

        Blockquote {
            quote: Document<'a>,
            author: Option<Text<'a>>,
        },

        Code {
            /// if None, try to guess lang (Language("Plain") to explicitly prevent syntax hl)
            lang: Option<Language>,
            code: Text<'a>,
            caption: Text<'a>,
        },

        ListUnordered(Vec<Text<'a>>),
        ListOrdered(Vec<Text<'a>>),
        ListDefinition(Vec<ListDefinitionItem<'a>>),
        ListTodo(Vec<ListTodoItem<'a>>),

        File {
            /// has at least one item
            media: Vec<Media>,
            caption: Option<Text<'a>>,
        },

        Callout {
            inner: Document<'a>,
            callout_type: CalloutType<'a>,
        },

        Table {
            header_row: Option<Vec<Text<'a>>>,
            header_column: Option<Vec<Text<'a>>>,
            cells: Vec<Vec<TableCell<'a>>>,
            // cells: Vec<TableCell2<'a>>,
            /// shown to everyone
            caption: Option<Document<'a>>,
            // /// shown to (only?) screen readers
            // alt: Option<Document<'a>>,
        },

        // katex? id prefer something standardizable/standardized though,
        // similar to mathml. but speccing that would be a lot of work...
        // NOTE: katex doesn't support tikz (for diagrams)
        // also everyone might want their own extensions lol, chemistry, music, etc
        Math(&'a str),
        // Aside(Box<Block<'a>>),
        // Interactive(BlockInteractive),
        // Embed stuff from somewhere else
        Embed(Embeddable),
    }

    struct ListDefinitionItem<'a> {
        term: Document<'a>,
        definition: Document<'a>,
    }

    struct ListTodoItem<'a> {
        content: Document<'a>,
    }

    struct TableCell<'a> {
        content: Document<'a>,
        colspan: u64,
        rowspan: u64,
    }

    // what if two cells try to be in the same place?
    struct TableCell2<'a> {
        content: Document<'a>,
        x: u64, // if none, use normal flow
        y: u64, // if none, use normal flow
        w: u64, // defaults to 1
        h: u64, // defaults to 1
    }

    enum TableCellAlign {
        Default,
        Left,
        Center,
        Right,
        Char(usize), // ?
    }

    // #[serde(untagged)]
    enum CalloutType<'a> {
        Custom {
            semantic: Option<CalloutTypeSemantic>,
            color: Option<Color>,
            label: Option<Document<'a>>,
            icon: Option<Media>,
        },
        Semantic {
            semantic: CalloutTypeSemantic,
        },
    }

    enum CalloutTypeSemantic {
        /// something worth pointing out
        Note,

        /// something with useful information
        Info,

        /// instructions or tips
        Help,

        /// very important to read, generic
        Important,

        /// very important to read, bad things happen if you don't
        Warning,

        /// very important to read, dangerous things happen if you don't
        Danger,

        /// something went wrong
        Error,

        /// something went right
        Success,
    }

    enum Embeddable {
        // Room(Room),
        // Thread(Thread),
        // User(User),
        // Message(Message),
        // Url(UrlEmbed),
    }

    // idk about *any* of these, just throwing random ideas out here
    // i'm probably not going to implement them

    // /// interactive, probably will be limited to bots
    // #[derive(Debug, Clone, PartialEq, Eq)]
    // pub enum BlockInteractive<'a> {
    //     /// a clickable button
    //     Button(Text<'a>, ButtonStyle),

    //     /// a text input
    //     Input(Text<'a>, InputStyle),

    //     /// collapseable summary and details
    //     Details(Text<'a>, Box<Block>),

    //     Radio,
    //     Checkbox,
    //     Form,
    // }

    // #[derive(Debug, Default, Clone, PartialEq, Eq)]
    // pub enum ButtonStyle {
    //     #[default]
    //     Default,
    //     Primary,
    //     Danger,
    // }

    // #[derive(Debug, Default, Clone, PartialEq, Eq)]
    // pub enum InputStyle {
    //     #[default]
    //     Singleline,

    //     // Multiline,
    //     // RichText,
    //     // Url,
    //     // Time,
    //     // Date,
    //     // DateTime,
    //     // Number,
    //     // File,
    //     // Color,
    //     // Search,
    //     // Select,

    //     // User,
    //     // MemberThread,
    //     // MemberRoom,
    //     // Room,
    //     // Message,
    //     // Thread,
    // }

    // /// for layout
    // #[derive(Debug, Clone, PartialEq, Eq)]
    // pub enum LayoutBlock<'a> {
    //     /// footnote/sidenote
    //     Aside(Box<Block<'a>>),
    //     Row(Vec<Block<'a>, StyleFlex>),
    //     Column(Vec<Block<'a>, StyleFlex>),
    //     Grid(Vec<Block<'a>>, StyleGrid),
    //     Box(Vec<Block<'a>>, StyleBox),
    // }

    // #[derive(Debug, Clone, PartialEq, Eq)]
    // #[cfg(feature = "formatting_extra")]
    // pub struct Blocks<'a>(Vec<Block<'a>>);

    #[allow(unused)]
    #[cfg(feature = "formatting_extra")]
    // i really should stop trying to overengineer this stuff
    // maybe i'll do a v1 static/no interaction version then a v2 format with interactivity
    // so that i at least have *some* way of formatting text for now
    //
    // another note: i want to let people add custom styles for user/room profiles.
    // i'd use css, but its kind of a pain to sanitize and scope properly. so i
    // might *also* end up making my own styling as well...
    //
    // plus theres the whole "maybe i want to let people run arbitrary code" thing.
    // might as well bite the bullet and figure out a way to safely let people do
    // *anything*, instead of incrementally/manually adding more and more features as
    // people ask for them.
    //
    /// the planning never ceases
    mod more_random_ideas {
        use crate::text::{Language, Text};

        /// a reference to another Block in a Document
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct BlockIdx(u64);

        /// block level formatting (WIP, new version?)
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Block<'a> {
            /// usually the root level container. allows combining multiple other block level elements.
            Container(Vec<u64>),

            /// inline text, can be a plain string
            Text(Text<'a>),

            H1(Text<'a>),
            H2(Text<'a>),
            H3(Text<'a>),
            H4(Text<'a>),
            H5(Text<'a>),
            H6(Text<'a>),
            Blockquote(BlockIdx),
            Code(Language, Text<'a>),
            ListUnordered(Vec<BlockIdx>),
            ListOrdered(Vec<BlockIdx>),
            ListDefinition(Vec<(BlockIdx, BlockIdx)>),
            ListCheckable(Vec<(BlockIdx, bool)>),
            Table(Vec<Vec<BlockIdx>>),
            Math(&'a str),

            /// for templates, a hole thats filled in later
            Data(Box<str>),

            /// causes/emits effects
            // bubble or capture?
            Effect(Effect),

            /// catches and transforms/handles effects
            // TODO: ?????
            // this is becoming very overcomplicated very quickly
            // i also want to limit the amount of "special" components and push more
            // stuff into "userspace", which is absolutely going to increase the
            // complexity here
            Handle(Input, Output),
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Input;

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Output;

        // also should look at https://adaptivecards.io/ for ideas
        // ...though i'll probably not copy it exactly
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum Effect {
            /// send an event to whoever sent this
            Event(Box<str>),

            /// do something to another thing
            Something(BlockIdx),
        }

        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct Document {
            /// is None, is assumed to be (thread, room, user)'s language
            pub lang: Option<Language>,

            /// first block is the root node. is a graph, not a tree.
            pub blocks: Vec<Block<'static>>,

            /// fills in Data
            pub data: std::collections::HashMap<Box<str>, BlockIdx>,
        }
    }
}

pub use doc::*;
