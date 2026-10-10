pub type NodeId = u32;

pub enum Kind {
    Element { name: Box<str>, attrs: Vec<(Box<str>, Box<str>)> },
    Text(String),
}

pub struct Node {
    pub kind: Kind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

pub struct Dom {
    nodes: Vec<Node>,
}

impl Dom {
    pub fn new(name: &str, attrs: Vec<(Box<str>, Box<str>)>) -> Dom {
        Dom { nodes: vec![Node { kind: Kind::Element { name: name.into(), attrs }, parent: None, children: Vec::new() }] }
    }

    pub const ROOT: NodeId = 0;

    fn push(&mut self, parent: NodeId, kind: Kind) -> NodeId {
        let id = self.nodes.len() as NodeId;
        self.nodes.push(Node { kind, parent: Some(parent), children: Vec::new() });
        self.nodes[parent as usize].children.push(id);
        id
    }

    pub fn element(&mut self, parent: NodeId, name: &str, attrs: Vec<(Box<str>, Box<str>)>) -> NodeId {
        self.push(parent, Kind::Element { name: name.into(), attrs })
    }

    pub fn text(&mut self, parent: NodeId, text: &str) -> NodeId {
        self.push(parent, Kind::Text(text.to_owned()))
    }

    pub fn add_class(&mut self, id: NodeId, class: &str) {
        if let Kind::Element { attrs, .. } = &mut self.nodes[id as usize].kind {
            match attrs.iter_mut().find(|(k, _)| &**k == "class") {
                Some((_, value)) => *value = format!("{value} {class}").into(),
                None => attrs.push(("class".into(), class.into())),
            }
        }
    }

    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn name(&self, id: NodeId) -> Option<&str> {
        match &self.node(id).kind {
            Kind::Element { name, .. } => Some(name),
            Kind::Text(_) => None,
        }
    }

    pub fn text_of(&self, id: NodeId) -> Option<&str> {
        match &self.node(id).kind {
            Kind::Text(text) => Some(text),
            Kind::Element { .. } => None,
        }
    }

    pub fn attr(&self, id: NodeId, key: &str) -> Option<&str> {
        match &self.node(id).kind {
            Kind::Element { attrs, .. } => attrs.iter().find(|(k, _)| &**k == key).map(|(_, v)| &**v),
            Kind::Text(_) => None,
        }
    }

    pub fn id(&self, id: NodeId) -> Option<&str> {
        self.attr(id, "id").filter(|v| !v.is_empty())
    }

    pub fn by_id(&self, value: &str) -> Option<NodeId> {
        (0..self.nodes.len() as NodeId).find(|&n| self.id(n) == Some(value))
    }

    pub fn from_xml(doc: &roxmltree::Document<'_>) -> Dom {
        let root = doc.root_element();
        let mut dom = Dom::new(root.tag_name().name(), attributes(root));
        dom.fill(Self::ROOT, root);
        dom
    }

    fn fill(&mut self, parent: NodeId, source: roxmltree::Node<'_, '_>) {
        for child in source.children() {
            if child.is_element() {
                let id = self.element(parent, child.tag_name().name(), attributes(child));
                self.fill(id, child);
            } else if child.is_text()
                && let Some(text) = child.text()
            {
                self.text(parent, text);
            }
        }
    }
}

pub fn attributes(node: roxmltree::Node<'_, '_>) -> Vec<(Box<str>, Box<str>)> {
    node.attributes().map(|a| (a.name().into(), a.value().into())).collect()
}

pub fn utf16_len(text: &str) -> u32 {
    text.encode_utf16().count() as u32
}
