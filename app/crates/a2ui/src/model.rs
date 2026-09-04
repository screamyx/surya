//! The typed A2UI component tree. One enum per catalog component the
//! renderer knows, an `Unknown` arm for everything else, and `Dynamic<T>`
//! for every property the spec lets an agent bind to the data model.

use std::collections::HashMap;

use serde_json::Value;

/// The id every surface's tree hangs off (A2UI: "exactly one component with
/// the ID `root`").
pub const ROOT_ID: &str = "root";

/// A client-side function call (`{"call": name, "args": {...}}`).
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCall {
    pub call: String,
    pub args: serde_json::Map<String, Value>,
}

/// A property that is a literal, a data-model path, or a function result.
#[derive(Debug, Clone, PartialEq)]
pub enum Dynamic<T> {
    Literal(T),
    /// JSON Pointer; relative (no leading `/`) inside a template scope.
    Path(String),
    Call(FunctionCall),
}

impl<T> Dynamic<T> {
    /// The bound path, when this property is a two-way binding target.
    pub fn path(&self) -> Option<&str> {
        match self {
            Dynamic::Path(p) => Some(p),
            _ => None,
        }
    }
}

/// How a container names its children: a fixed id list, or one template
/// component instantiated per item of a data-model array.
#[derive(Debug, Clone, PartialEq)]
pub enum ChildList {
    Static(Vec<String>),
    Template { component_id: String, path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Justify {
    #[default]
    Start,
    Center,
    End,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextVariant {
    H1,
    H2,
    H3,
    H4,
    H5,
    Caption,
    #[default]
    Body,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageVariant {
    Icon,
    Avatar,
    SmallFeature,
    #[default]
    MediumFeature,
    LargeFeature,
    Header,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFit {
    Contain,
    Cover,
    #[default]
    Fill,
    None,
    ScaleDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Default,
    Primary,
    Borderless,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFieldVariant {
    #[default]
    ShortText,
    LongText,
    Number,
    Obscured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// A server event the card dispatches (`action.event`).
#[derive(Debug, Clone, PartialEq)]
pub struct EventAction {
    pub name: String,
    /// Values are literals or `{path}` / `{call}` objects, resolved at
    /// dispatch time against the card's current data model.
    pub context: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Event(EventAction),
    Function(FunctionCall),
    /// No usable `action` — the button renders but a press does nothing.
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub title: Dynamic<String>,
    pub child: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentKind {
    Text {
        text: Dynamic<String>,
        variant: TextVariant,
    },
    Image {
        url: Dynamic<String>,
        fit: ImageFit,
        variant: ImageVariant,
        description: Option<Dynamic<String>>,
    },
    Button {
        child: String,
        variant: ButtonVariant,
        action: Action,
    },
    TextField {
        label: Dynamic<String>,
        value: Option<Dynamic<String>>,
        variant: TextFieldVariant,
    },
    CheckBox {
        label: Dynamic<String>,
        value: Dynamic<bool>,
    },
    Row {
        children: ChildList,
        justify: Justify,
        align: Align,
    },
    Column {
        children: ChildList,
        justify: Justify,
        align: Align,
    },
    List {
        children: ChildList,
        direction: Axis,
        align: Align,
    },
    Card {
        child: String,
    },
    Divider {
        axis: Axis,
    },
    Tabs {
        tabs: Vec<Tab>,
    },
    /// A catalog component this renderer does not draw (Icon, Video, Modal,
    /// Slider, ChoicePicker, DateTimeInput, AudioPlayer, or a workspace
    /// catalog's own). Rendered as a labelled box, never a crash.
    Unknown {
        name: String,
        raw: Value,
    },
}

impl ComponentKind {
    /// The catalog name, for labels and diagnostics.
    pub fn name(&self) -> &str {
        match self {
            ComponentKind::Text { .. } => "Text",
            ComponentKind::Image { .. } => "Image",
            ComponentKind::Button { .. } => "Button",
            ComponentKind::TextField { .. } => "TextField",
            ComponentKind::CheckBox { .. } => "CheckBox",
            ComponentKind::Row { .. } => "Row",
            ComponentKind::Column { .. } => "Column",
            ComponentKind::List { .. } => "List",
            ComponentKind::Card { .. } => "Card",
            ComponentKind::Divider { .. } => "Divider",
            ComponentKind::Tabs { .. } => "Tabs",
            ComponentKind::Unknown { name, .. } => name,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub id: String,
    pub kind: ComponentKind,
    /// Flex weight inside a Row or Column (`weight: 1` shares the axis).
    pub weight: Option<f32>,
}

/// One parsed A2UI surface: the component map, its initial data model, and
/// whatever went wrong on the way in.
#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    /// The `surfaceId`; also the id a button press reports back under.
    pub id: String,
    pub catalog_id: String,
    pub components: HashMap<String, Component>,
    /// The data model as sent (`updateDataModel` folded in). Live edits
    /// happen on a [`crate::state::CardState`] copy.
    pub data: Value,
    /// Parse diagnostics. A card with errors still renders what it can; a
    /// card with no `root` renders only these.
    pub errors: Vec<String>,
}

impl Card {
    pub fn get(&self, id: &str) -> Option<&Component> {
        self.components.get(id)
    }

    pub fn root(&self) -> Option<&Component> {
        self.get(ROOT_ID)
    }

    /// Ids in a stable order, for tests and diagnostics.
    pub fn component_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.components.keys().map(String::as_str).collect();
        ids.sort_unstable();
        ids
    }
}
