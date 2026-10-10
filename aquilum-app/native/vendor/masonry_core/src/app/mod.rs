// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

//! Types needed for running a Masonry app.

mod layer_stack;
mod render_root;
mod visual_layers;

pub use render_root::{RenderRoot, RenderRootOptions, RenderRootSignal, WindowSizePolicy};
pub use visual_layers::{VisualLayer, VisualLayerKind, VisualLayerPlan};

pub(crate) use render_root::{MutateCallback, RenderRootState};
