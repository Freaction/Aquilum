use aquilum_core::settings::models::AppConfig;
use masonry::app::RenderRoot;
use masonry::kurbo::Point;

use super::Scene;
use crate::ui::settings::schema::{Context, Section};
use crate::ui::settings;
use crate::ui::snapshot::shell_with;

fn show(root: &mut RenderRoot, config: &AppConfig, section: Section) {
    let (layer, _) = settings::dialog(config, &Context::new(1.0), section);
    root.add_layer(layer, Point::ORIGIN);
}

pub(super) fn primary(scene: &Scene, config: &AppConfig) {
    let (mut root, _) = scene.shell();
    show(&mut root, config, Section::Ui);
    scene.save(&mut root, "primary");
}

pub(super) fn snapshots(scene: &Scene, config: &AppConfig) {
    for scale in [1.75, 1.75 * 1.05] {
        let (mut root, _) = shell_with(scene.vault, scene.data, scale, scale);
        show(&mut root, config, Section::Ui);
        scene.save(&mut root, &format!("scaled-{}", (scale * 100.0).round()));
    }
    for section in [Section::Ui, Section::Editor, Section::Shortcuts, Section::Mcp, Section::System, Section::Trash] {
        let (mut root, _) = scene.shell();
        show(&mut root, config, section);
        scene.save(&mut root, &format!("settings-{}", section.key()));
    }
}
