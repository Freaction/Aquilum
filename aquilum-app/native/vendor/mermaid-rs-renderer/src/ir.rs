use std::collections::{BTreeMap, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    TopDown,
    LeftRight,
    BottomTop,
    RightLeft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramKind {
    Flowchart,
    Class,
    State,
    Sequence,
    Er,
    Pie,
    Mindmap,
    Journey,
    Timeline,
    Gantt,
    Requirement,
    GitGraph,
    C4,
    Sankey,
    Quadrant,
    ZenUML,
    Block,
    Packet,
    Kanban,
    Architecture,
    Radar,
    Treemap,
    XYChart,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceFrameKind {
    Alt,
    Opt,
    Loop,
    Par,
    Rect,
    Critical,
    Break,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceEvent {
    Message(usize),
    Note(usize),
    FrameStart(usize),
    FrameSection(usize),
    FrameEnd(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceNotePosition {
    LeftOf,
    RightOf,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateNotePosition {
    LeftOf,
    RightOf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequenceActivationKind {
    Activate,
    Deactivate,
}

#[derive(Debug, Clone)]
pub struct SequenceActivation {
    pub participant: String,
    pub index: usize,
    pub kind: SequenceActivationKind,
}

#[derive(Debug, Clone)]
pub struct SequenceNote {
    pub position: SequenceNotePosition,
    pub participants: Vec<String>,
    pub label: String,
    pub index: usize,
}

#[derive(Debug, Clone)]
pub struct PieSlice {
    pub label: String,
    pub value: f32,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GanttStatus {
    Done,
    Active,
    Crit,
    Milestone,
}


#[derive(Debug, Clone)]
pub struct GanttTask {
    pub id: String,
    pub label: String,
    pub start: Option<String>,
    pub duration: Option<String>,
    pub after: Option<String>,
    pub section: Option<String>,
    pub status: Option<GanttStatus>,
}












#[derive(Debug, Clone)]
pub struct SequenceBox {
    pub label: Option<String>,
    pub color: Option<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct StateNote {
    pub position: StateNotePosition,
    pub target: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct SequenceFrameSection {
    pub label: Option<String>,
    pub start_idx: usize,
    pub end_idx: usize,
}

#[derive(Debug, Clone)]
pub struct SequenceFrame {
    pub kind: SequenceFrameKind,
    pub sections: Vec<SequenceFrameSection>,
    pub start_idx: usize,
    pub end_idx: usize,
}

impl Direction {
    pub fn from_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" | "TB" => Some(Self::TopDown),
            "BT" => Some(Self::BottomTop),
            "LR" => Some(Self::LeftRight),
            "RL" => Some(Self::RightLeft),
            _ => None,
        }
    }

    pub fn from_timeline_token(token: &str) -> Option<Self> {
        let upper = token.to_ascii_uppercase();
        match upper.as_str() {
            "TD" => Some(Self::TopDown),
            "LR" => Some(Self::LeftRight),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Node {
    pub id: String,
    pub label: String,
    pub shape: NodeShape,
    pub value: Option<f32>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NodeLink {
    pub url: String,
    pub title: Option<String>,
    pub target: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub label: Option<String>,
    pub start_label: Option<String>,
    pub end_label: Option<String>,
    pub directed: bool,
    pub arrow_start: bool,
    pub arrow_end: bool,
    pub arrow_start_kind: Option<EdgeArrowhead>,
    pub arrow_end_kind: Option<EdgeArrowhead>,
    pub start_decoration: Option<EdgeDecoration>,
    pub end_decoration: Option<EdgeDecoration>,
    pub style: EdgeStyle,
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeStyle {
    Solid,
    Dotted,
    Thick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDecoration {
    Circle,
    Cross,
    Diamond,
    DiamondFilled,
    // Crow's foot notation for ER diagrams
    CrowsFootOne,      // || exactly one
    CrowsFootZeroOne,  // o| zero or one
    CrowsFootMany,     // |{ one or many
    CrowsFootZeroMany, // o{ zero or many
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeArrowhead {
    OpenTriangle,
    ClassDependency,
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub id: Option<String>,
    pub label: String,
    pub nodes: Vec<String>,
    pub direction: Option<Direction>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub kind: DiagramKind,
    pub direction: Direction,
    pub nodes: BTreeMap<String, Node>,
    pub node_order: HashMap<String, usize>,
    pub edges: Vec<Edge>,
    pub subgraphs: Vec<Subgraph>,
    pub sequence_participants: Vec<String>,
    pub sequence_frames: Vec<SequenceFrame>,
    pub sequence_events: Vec<SequenceEvent>,
    pub sequence_notes: Vec<SequenceNote>,
    pub sequence_activations: Vec<SequenceActivation>,
    pub sequence_autonumber: Option<usize>,
    pub sequence_boxes: Vec<SequenceBox>,
    pub state_notes: Vec<StateNote>,
    pub pie_slices: Vec<PieSlice>,
    pub pie_title: Option<String>,
    pub pie_show_data: bool,
    pub gantt_tasks: Vec<GanttTask>,
    pub gantt_title: Option<String>,
    pub gantt_sections: Vec<String>,
    pub gantt_display_mode: Option<String>,
    pub class_defs: HashMap<String, NodeStyle>,
    pub node_classes: HashMap<String, Vec<String>>,
    pub node_styles: HashMap<String, NodeStyle>,
    pub subgraph_styles: HashMap<String, NodeStyle>,
    pub subgraph_classes: HashMap<String, Vec<String>>,
    pub node_links: HashMap<String, NodeLink>,
    pub edge_styles: HashMap<usize, EdgeStyleOverride>,
    pub edge_style_default: Option<EdgeStyleOverride>,
    pub mindmap: MindmapData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeShape {
    Rectangle,
    ForkJoin,
    RoundRect,
    Stadium,
    Subroutine,
    Cylinder,
    ActorBox,
    Circle,
    DoubleCircle,
    Diamond,
    Hexagon,
    Parallelogram,
    ParallelogramAlt,
    Trapezoid,
    TrapezoidAlt,
    Asymmetric,
    MindmapDefault,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindmapNodeType {
    Default,
    RoundedRect,
    Rect,
    Circle,
    Cloud,
    Bang,
    Hexagon,
}

#[derive(Debug, Clone)]
pub struct MindmapNode {
    pub id: String,
    pub label: String,
    pub level: usize,
    pub section: Option<usize>,
    pub node_type: MindmapNodeType,
    pub icon: Option<String>,
    pub class: Option<String>,
    pub children: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct MindmapData {
    pub nodes: Vec<MindmapNode>,
    pub root_id: Option<String>,
}








impl Graph {
    pub fn new() -> Self {
        Self {
            kind: DiagramKind::Flowchart,
            direction: Direction::TopDown,
            nodes: BTreeMap::new(),
            node_order: HashMap::new(),
            edges: Vec::new(),
            subgraphs: Vec::new(),
            sequence_participants: Vec::new(),
            sequence_frames: Vec::new(),
            sequence_events: Vec::new(),
            sequence_notes: Vec::new(),
            sequence_activations: Vec::new(),
            sequence_autonumber: None,
            sequence_boxes: Vec::new(),
            state_notes: Vec::new(),
            pie_slices: Vec::new(),
            pie_title: None,
            pie_show_data: false,
            gantt_tasks: Vec::new(),
            gantt_title: None,
            gantt_sections: Vec::new(),
            gantt_display_mode: None,
            class_defs: HashMap::new(),
            node_classes: HashMap::new(),
            node_styles: HashMap::new(),
            subgraph_styles: HashMap::new(),
            subgraph_classes: HashMap::new(),
            node_links: HashMap::new(),
            edge_styles: HashMap::new(),
            edge_style_default: None,
            mindmap: MindmapData::default(),
        }
    }

    pub fn ensure_node(&mut self, id: &str, label: Option<String>, shape: Option<NodeShape>) {
        let is_new = !self.nodes.contains_key(id);
        let entry = self.nodes.entry(id.to_string()).or_insert(Node {
            id: id.to_string(),
            label: id.to_string(),
            shape: NodeShape::Rectangle,
            value: None,
            icon: None,
        });
        if is_new {
            let order = self.node_order.len();
            self.node_order.insert(id.to_string(), order);
        }
        if let Some(label) = label {
            entry.label = label;
        }
        if let Some(shape) = shape {
            entry.shape = shape;
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct NodeStyle {
    pub fill: Option<String>,
    pub stroke: Option<String>,
    pub text_color: Option<String>,
    pub stroke_width: Option<f32>,
    pub stroke_dasharray: Option<String>,
    pub line_color: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct EdgeStyleOverride {
    pub stroke: Option<String>,
    pub stroke_width: Option<f32>,
    pub dasharray: Option<String>,
    pub label_color: Option<String>,
}

impl Default for Graph {
    fn default() -> Self {
        Self::new()
    }
}
