// Copyright 2025 the Xilem Authors
// SPDX-License-Identifier: Apache-2.0

use std::any::TypeId;

use crate::core::{Property, UpdateCtx};
use crate::layout::Length;
use crate::peniko::Color;

/// The radius of a slider's thumb.
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct ThumbRadius(pub Length);

impl Property for ThumbRadius {
    fn static_default() -> &'static Self {
        static DEFAULT: ThumbRadius = ThumbRadius(Length::ZERO);
        &DEFAULT
    }
}

impl ThumbRadius {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_layout();
        }
    }
}

/// The color of a slider's thumb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ThumbColor(pub Color);

impl Property for ThumbColor {
    fn static_default() -> &'static Self {
        static DEFAULT: ThumbColor = ThumbColor(Color::WHITE);
        &DEFAULT
    }
}

impl Default for ThumbColor {
    fn default() -> Self {
        *Self::static_default()
    }
}

impl ThumbColor {
    /// Helper function to be called in [`Widget::property_changed`](crate::core::Widget::property_changed).
    pub fn prop_changed(ctx: &mut UpdateCtx<'_>, property_type: TypeId) {
        if property_type == TypeId::of::<Self>() {
            ctx.request_paint_only();
        }
    }
}
