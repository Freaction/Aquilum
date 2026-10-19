use masonry::app::RenderRoot;

use super::App;
use super::graph_page::GraphState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Moment {
    Hide,
    Tab,
    Vault,
}

use Moment::{Hide, Tab, Vault};

struct Cache {
    name: &'static str,
    when: &'static [Moment],
    clear: fn(&mut App, &mut RenderRoot),
}

const CATALOG: &[Cache] = &[
    Cache { name: "картинки", when: &[Hide, Tab, Vault], clear: |_, _| crate::images::trim() },
    Cache { name: "недавние вставки", when: &[Hide, Vault], clear: |_, _| super::embeds::forget_recent() },
    Cache { name: "узоры обложек", when: &[Hide, Vault], clear: |_, _| crate::cover::trim() },
    Cache { name: "плитка обложки «город»", when: &[Hide, Vault], clear: |_, _| crate::cover::forget_tile() },
    Cache { name: "лучи и звезда книжной обложки", when: &[Hide, Vault], clear: |_, _| crate::ui::book::forget_art() },
    Cache { name: "буферы рендера и атлас глифов", when: &[Hide], clear: |_, _| crate::render::trim_soon() },
    Cache { name: "кадры и подписи графа", when: &[Hide], clear: |app, root| app.drop_graph_frames(root) },
    Cache { name: "панель ссылок и анализа", when: &[Tab, Vault], clear: |app, _| (app.listing, app.panel_body, app.panel_key) = (None, None, None) },
    Cache { name: "история в панели", when: &[Tab, Vault], clear: |app, _| (app.history, app.history_body) = (None, None) },
    Cache { name: "просмотр версии", when: &[Tab, Vault], clear: |app, root| app.drop_version(root) },
    Cache { name: "версии закрытых вкладок", when: &[Tab], clear: |app, _| {
        let tabs = &app.tabs;
        app.opened_versions.retain(|id, _| tabs.tabs().iter().any(|tab| tab.id == *id));
    } },
    Cache { name: "граф", when: &[Tab], clear: |app, root| app.release_graph(root) },
    Cache { name: "граф ссылок для анализа", when: &[Tab, Vault], clear: |app, _| app.workspace.core.search.forget_analysis() },
    Cache { name: "Wikixiv", when: &[Tab, Vault], clear: |app, _| app.workspace.core.wikixiv.forget() },
    Cache { name: "граф базы", when: &[Vault], clear: |app, _| app.graph = GraphState::default() },
    Cache { name: "открытые версии", when: &[Vault], clear: |app, _| {
        app.opened_versions.clear();
        app.version_notice = None;
    } },
    Cache { name: "домашняя страница", when: &[Vault], clear: |app, _| app.home = None },
    Cache { name: "вложения базы", when: &[Vault], clear: |_, _| aquilum_core::files::attachments::forget_attachments() },
    Cache { name: "освобождённая память кучи", when: &[Hide, Tab, Vault], clear: |_, _| crate::system::release_memory() },
];

impl App {
    pub(crate) fn forget(&mut self, moment: Moment, root: &mut RenderRoot) {
        for cache in CATALOG.iter().filter(|cache| cache.when.contains(&moment)) {
            let _zone = aq_trace::zone(cache.name);
            (cache.clear)(self, root);
        }
    }
}
