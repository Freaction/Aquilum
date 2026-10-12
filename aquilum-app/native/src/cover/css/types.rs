#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Val {
    pub num: f64,
    pub px: f64,
    pub pct: f64,
    pub deg: f64,
}

impl Val {
    pub(super) fn add(self, o: Val, sign: f64) -> Val {
        Val { num: self.num + sign * o.num, px: self.px + sign * o.px, pct: self.pct + sign * o.pct, deg: self.deg + sign * o.deg }
    }

    pub(super) fn scale(self, k: f64) -> Val {
        Val { num: self.num * k, px: self.px * k, pct: self.pct * k, deg: self.deg * k }
    }

    pub(super) fn is_number(&self) -> bool {
        self.px == 0.0 && self.pct == 0.0 && self.deg == 0.0
    }

    pub fn length(&self, reference: f64) -> f64 {
        self.px + self.num + self.pct / 100.0 * reference
    }

    pub fn angle(&self) -> f64 {
        self.deg + self.pct / 100.0 * 360.0 + self.num
    }
}

pub type Rgba = [f64; 4];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Extent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RadialSize {
    Extent(Extent),
    Explicit(Val, Val),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Linear { angle: Option<f64>, corner: Option<(f64, f64)> },
    Radial { circle: bool, size: RadialSize, at: (Val, Val) },
    Conic { from: f64, at: (Val, Val) },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Stop {
    pub color: Rgba,
    pub pos: Option<Val>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Gradient {
    pub kind: Kind,
    pub repeating: bool,
    pub stops: Vec<Stop>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub gradient: Gradient,
    pub position: (Val, Val),
    pub size: Option<(Val, Val)>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Background {
    pub color: Rgba,
    pub layers: Vec<Layer>,
}

#[derive(Clone, Debug, Default)]
pub struct Pattern {
    pub base: Background,
    pub overlay: Background,
}
