//! Retained native collection states. JSX content is mounted through the NSH bridge.
use gpui_kit::IntoElement;
#[path = "collections/command.rs"]
mod command;
#[path = "collections/list.rs"]
mod list;
#[path = "collections/pagination.rs"]
mod pagination;
#[path = "collections/picker.rs"]
mod picker;
#[path = "collections/schema.rs"]
mod schema;
#[path = "collections/table.rs"]
mod table;
#[path = "collections/tree.rs"]
mod tree;

#[path = "collections/parts.rs"]
mod parts;

crate::registration::native_leaf!(NativeListLoading(ListLoadingProps,context,cx)=>gpui_kit::component::list::Loading; {});

pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<list::NativeList>("gpui-component.List", "List", list::NativeList::new);
    registry.register::<table::NativeTable>("gpui-component.DataTable", "DataTable", |context| {
        table::NativeTable::new(context, true)
    });
    registry.register::<table::NativeBasicTable>(
        "gpui-component.Table",
        "Table",
        table::NativeBasicTable::new,
    );
    registry.register::<tree::NativeTree>("gpui-component.Tree", "Tree", tree::NativeTree::new);
    registry.register::<picker::NativeCombobox>(
        "gpui-component.Combobox",
        "Combobox",
        picker::NativeCombobox::new,
    );
    registry.register::<command::NativeCommand>(
        "gpui-component.Command",
        "Command",
        command::NativeCommand::new,
    );
    registry.register::<pagination::NativePagination>(
        "gpui-component.Pagination",
        "Pagination",
        pagination::NativePagination::new,
    );
    registry.register::<parts::TablePart<0>>(
        "gpui-component.TableHeader",
        "TableHeader",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<1>>(
        "gpui-component.TableBody",
        "TableBody",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<2>>(
        "gpui-component.TableFooter",
        "TableFooter",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<3>>(
        "gpui-component.TableRow",
        "TableRow",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<4>>(
        "gpui-component.TableHead",
        "TableHead",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<5>>(
        "gpui-component.TableCell",
        "TableCell",
        parts::TablePart::new,
    );
    registry.register::<parts::TablePart<6>>(
        "gpui-component.TableCaption",
        "TableCaption",
        parts::TablePart::new,
    );
    registry.register::<parts::NativeListItem>(
        "gpui-component.ListItem",
        "ListItem",
        parts::NativeListItem::new,
    );
    registry.register::<parts::NativeSeparator>(
        "gpui-component.ListSeparatorItem",
        "ListSeparatorItem",
        parts::NativeSeparator::new,
    );
    registry.register::<parts::NativeSearchableItem>(
        "gpui-component.SearchableListItemElement",
        "SearchableListItemElement",
        parts::NativeSearchableItem::new,
    );
    registry.register::<NativeListLoading>(
        "gpui-component.ListLoading",
        "ListLoading",
        NativeListLoading::new,
    );
    for (name, controls) in [
        (
            "gpui-component.List",
            vec![
                ("selectionChange", vec!["selectedId"]),
                ("queryChange", vec!["query"]),
            ],
        ),
        (
            "gpui-component.Tree",
            vec![
                ("selectionChange", vec!["selectedId"]),
                ("expandedChange", vec!["expandedIds"]),
            ],
        ),
        (
            "gpui-component.Command",
            vec![
                ("selectionChange", vec!["selectedId"]),
                ("queryChange", vec!["query"]),
            ],
        ),
        (
            "gpui-component.Combobox",
            vec![
                ("change", vec!["values", "query"]),
                ("confirm", vec!["values", "query"]),
            ],
        ),
        (
            "gpui-component.DataTable",
            vec![("selectionChange", vec!["selection"])],
        ),
        ("gpui-component.Pagination", vec![("change", vec!["page"])]),
    ] {
        let controls = controls
            .iter()
            .map(|(event, fields)| (*event, fields.as_slice()))
            .collect::<Vec<_>>();
        registry.metadata().native_controlled(name, &controls);
    }
}
