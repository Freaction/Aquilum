# Bases and Kanban

The native Masonry app opens Obsidian Bases (`.base`) as board tabs. A board reads note metadata from the shared search index, evaluates the supported view filters, groups rows, and keeps configured empty columns. The board page and selected view are restored with the workspace session.

## Supported view behavior

The implementation supports YAML `views` with `type: kanban`, `name`, `filters`, `groupBy`, `groupOrder`, `sort`, and `limit`. Top-level and per-view filters can use equality expressions, `file.hasTag("tag")`, and nested `and`, `or`, and `not` groups. Equality values may be strings, numbers, booleans, or null. Properties include note frontmatter keys, `note["key with spaces"]`, and `file.path`, `file.name`, `file.ext`, `file.folder`, and `file.tags`.

Unsupported filter operators and properties produce an error instead of silently changing board membership. Formula evaluation and non-Kanban views are not implemented. `file.tags` combines frontmatter tags with Markdown hashtags; `file.hasTag` also matches descendants of a hierarchical tag. Missing or empty group values appear in an unassigned column.

## Safe card changes

Creating a card writes a new Markdown file through the shared file gate. Moving a card reads its current contents, changes only the configured grouping frontmatter value, and writes with the snapshot hash. A concurrent disk change causes one fresh read and retry; if the card's group changed, the move stops with a conflict. After a successful move, an already-open note is reconciled through the document service so its Y.Doc is not replaced.

Cards can be renamed through the shared file rename gate, which updates links and open document paths. Deletion asks for confirmation and moves the note to the workspace trash.

A view is read-only when its group property cannot be safely edited or its filters make membership uncertain. Derived `file.*` fields are read-only. Unsupported or malformed `.base` files are never rewritten by the board.

## Source preservation

The parser retains the original `.base` source and unknown YAML fields. Board operations do not serialize the `.base` definition, so unsupported fields remain intact. The search index supplies typed YAML frontmatter values; quoted strings stay strings.
