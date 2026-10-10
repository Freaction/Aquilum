use super::{BaseDefinition, BaseError};

impl BaseDefinition {
    pub fn parse(raw: &str) -> Result<Self, BaseError> {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(raw)
            .map_err(|error| BaseError::InvalidYaml(error.to_string()))?;
        let mut definition: Self = serde_yaml_ng::from_value(value)
            .map_err(|error| BaseError::InvalidYaml(error.to_string()))?;
        definition.source = raw.to_owned();
        Ok(definition)
    }
}

#[cfg(test)]
mod tests {
    use crate::bases::BaseDefinition;

    #[test]
    fn parses_views_nested_filters_and_unknown_values_without_losing_source() {
        let raw = concat!(
            "filters:\r\n  and:\r\n    - 'project == \"Alpha: #1\"'\r\n",
            "    - or:\r\n      - 'file.hasTag(\"work\")'\r\n",
            "      - not:\r\n        - 'status == \"Done\"'\r\n",
            "formulas:\r\n  effort: 'hours * 2'\r\n",
            "custom:\r\n  nested: [one, {two: true}]\r\n",
            "views:\r\n  - type: kanban\r\n    name: 'Board: #1'\r\n",
            "    groupBy: {property: note.status, direction: DESC}\r\n",
            "    groupOrder: [Todo, Done, null]\r\n",
            "    sort: [{property: file.name, direction: ASC}]\r\n",
            "    limit: 20\r\n    filters: 'status == \"Todo\"'\r\n",
            "    card: {cover: true}\r\n",
            "  - type: table\r\n    name: All\r\n"
        );
        let base = BaseDefinition::parse(raw).unwrap();
        assert_eq!(base.source, raw);
        assert_eq!(base.views.len(), 2);
        let view = &base.views[0];
        assert_eq!(view.name, "Board: #1");
        assert_eq!(view.view_type, "kanban");
        assert_eq!(view.group_by.as_ref().unwrap().property, "note.status");
        assert_eq!(view.group_order.as_ref().unwrap().len(), 3);
        assert_eq!(view.sort[0].property, "file.name");
        assert_eq!(view.limit, Some(20));
        assert!(base.filters.is_some());
        assert!(view.filters.is_some());
        assert!(base.extra.contains_key("formulas"));
        assert!(base.extra.contains_key("custom"));
        assert!(view.extra.contains_key("card"));
    }

    #[test]
    fn rejects_malformed_yaml_duplicate_keys_and_invalid_known_shapes() {
        for raw in [
            "views: [",
            "views: []\nviews: []",
            "views: [{type: kanban, groupBy: status}]",
            "views: [{type: kanban, limit: -1}]",
            "views: [{type: kanban, sort: [{property: status, direction: sideways}]}]",
        ] {
            assert!(BaseDefinition::parse(raw).is_err(), "{raw}");
        }
    }
}
