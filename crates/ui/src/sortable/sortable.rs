use std::rc::Rc;

use gpui::{
    div, prelude::FluentBuilder, px, AnyElement, App, AppContext, Axis, Context, DragMoveEvent,
    ElementId, Entity, InteractiveElement, IntoElement, ParentElement, Pixels, Refineable, Render,
    RenderOnce, Size, StatefulInteractiveElement, Styled, StyleRefinement,
    Subscription, Window,
};

use crate::ActiveTheme;

/// Data carried during a drag operation from a [`Sortable`] list.
///
/// The type parameter `T` determines cross-list compatibility:
/// all `Sortable<T>` instances with the same `T` accept each other's items.
#[derive(Clone)]
pub struct SortableDragData<T: Clone + 'static> {
    pub item: T,
    pub from_index: usize,
    pub source: Entity<SortableState<T>>,
    render_item: Rc<dyn Fn(&T, usize, &Window, &App) -> AnyElement>,
}

impl<T: Clone + 'static> Render for SortableDragData<T> {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let drag_size = self.source.read(cx).drag_item_size;
        div()
            .opacity(0.85)
            .when_some(drag_size, |d, size| d.w(size.width).h(size.height))
            .child((self.render_item)(&self.item, self.from_index, window, cx))
    }
}

/// State entity for a [`Sortable`] list.
pub struct SortableState<T: Clone + 'static> {
    items: Vec<T>,
    dragging_from_index: Option<usize>,
    drag_item_size: Option<Size<Pixels>>,
    /// Where the dragged item would be inserted (computed from cursor position).
    drop_target_index: Option<usize>,
    _drag_subscription: Option<Subscription>,
}

impl<T: Clone + 'static> SortableState<T> {
    pub fn new(items: Vec<T>) -> Self {
        Self {
            items,
            dragging_from_index: None,
            drag_item_size: None,
            drop_target_index: None,
            _drag_subscription: None,
        }
    }

    pub fn items(&self) -> &[T] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut Vec<T> {
        &mut self.items
    }

    pub fn set_items(&mut self, items: Vec<T>, cx: &mut Context<Self>) {
        self.items = items;
        self.dragging_from_index = None;
        self.drag_item_size = None;
        self.drop_target_index = None;
        self._drag_subscription = None;
        cx.notify();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A reorderable list component.
///
/// Items can be reordered within the list by dragging. Multiple `Sortable`
/// instances sharing the same item type `T` support cross-list drag-and-drop
/// automatically.
///
/// # Example
///
/// ```ignore
/// Sortable::new(
///     "my-list",
///     state,
///     |item: &MyItem| item.id.into(),
///     |item, _ix, _window, _cx| {
///         div().child(item.name.clone()).into_any_element()
///     },
/// )
/// .axis(Axis::Vertical)
/// .gap(px(8.))
/// .on_reorder(|from, to, _window, _cx| {
///     println!("moved from {from} to {to}");
/// })
/// ```
#[derive(IntoElement)]
pub struct Sortable<T: Clone + 'static> {
    id: ElementId,
    state: Entity<SortableState<T>>,
    axis: Axis,
    item_id: Rc<dyn Fn(&T) -> ElementId>,
    render_item: Rc<dyn Fn(&T, usize, &Window, &App) -> AnyElement>,
    on_reorder: Option<Rc<dyn Fn(usize, usize, &mut Window, &mut App)>>,
    on_insert: Option<Rc<dyn Fn(T, usize, Entity<SortableState<T>>, &mut Window, &mut App)>>,
    gap: Pixels,
    disabled: bool,
    style: StyleRefinement,
}

impl<T: Clone + 'static> Sortable<T> {
    pub fn new(
        id: impl Into<ElementId>,
        state: Entity<SortableState<T>>,
        item_id: impl Fn(&T) -> ElementId + 'static,
        render_item: impl Fn(&T, usize, &Window, &App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            state,
            axis: Axis::Vertical,
            item_id: Rc::new(item_id),
            render_item: Rc::new(render_item),
            on_reorder: None,
            on_insert: None,
            gap: px(4.0),
            disabled: false,
            style: StyleRefinement::default(),
        }
    }

    /// Set the layout axis. `Vertical` stacks items top-to-bottom (default),
    /// `Horizontal` arranges them left-to-right.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Set the gap between items.
    pub fn gap(mut self, gap: impl Into<Pixels>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Disable drag-and-drop (items render but cannot be reordered).
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Called after an item is reordered within this list.
    ///
    /// The state has already been updated optimistically when this fires.
    /// Arguments: `(from_index, to_index, window, cx)`.
    pub fn on_reorder(
        mut self,
        f: impl Fn(usize, usize, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_reorder = Some(Rc::new(f));
        self
    }

    /// Called after an item is inserted from another `Sortable`.
    ///
    /// Both source and target states have been updated optimistically.
    /// Arguments: `(item, insert_index, source_state, window, cx)`.
    pub fn on_insert(
        mut self,
        f: impl Fn(T, usize, Entity<SortableState<T>>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_insert = Some(Rc::new(f));
        self
    }
}

impl<T: Clone + 'static> Styled for Sortable<T> {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl<T: Clone + 'static> RenderOnce for Sortable<T> {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let items = self.state.read(cx).items.clone();
        let dragging_from = self.state.read(cx).dragging_from_index;
        let drop_target = self.state.read(cx).drop_target_index;
        let axis = self.axis;
        let gap = self.gap;
        let state = self.state;
        let item_id_fn = self.item_id;
        let render_fn = self.render_item;
        let on_reorder = self.on_reorder;
        let on_insert = self.on_insert;
        let drag_border = cx.theme().drag_border;
        let ghost_bg = cx.theme().muted;
        let border_color = cx.theme().border;
        let user_style = self.style;
        let state_entity_id = state.entity_id();
        let item_count = items.len();

        let mut container = div()
            .id(self.id)
            .flex()
            .when(axis == Axis::Vertical, |d| d.flex_col())
            .when(items.is_empty(), |d| {
                if axis == Axis::Vertical {
                    d.min_h(gap * 2.)
                } else {
                    d.min_w(gap * 2.)
                }
            })
            .map(|mut d| {
                d.style().refine(&user_style);
                d
            });

        if self.disabled {
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    container = container.child(spacer(axis, gap));
                }
                let rendered = (render_fn)(item, index, window, cx);
                container = container.child(rendered);
            }
            return container;
        }

        // Container-level drag_move: compute drop target from cursor position.
        // Divides the container evenly among items and uses the cursor's
        // position within each slot to determine the insertion index.
        let state_for_container = state.clone();
        container = container
            .on_drag_move::<SortableDragData<T>>({
                move |event: &DragMoveEvent<SortableDragData<T>>, _window, cx| {
                    let cursor = event.event.position;
                    let bounds = event.bounds;

                    // Clear drop target if cursor left this container.
                    if !bounds.contains(&cursor) {
                        state_for_container.update(cx, |s, cx| {
                            if s.drop_target_index.is_some() {
                                s.drop_target_index = None;
                                cx.notify();
                            }
                        });
                        return;
                    }

                    // Compute target index by dividing container evenly among items.
                    let target = if item_count == 0 {
                        0
                    } else if axis == Axis::Vertical {
                        let relative = (cursor.y - bounds.origin.y).max(px(0.));
                        let slot_height = bounds.size.height / item_count as f32;
                        let slot = (relative / slot_height).floor() as usize;
                        let within_slot = relative - slot_height * slot as f32;
                        let insert_index = if within_slot > slot_height / 2. {
                            slot + 1
                        } else {
                            slot
                        };
                        insert_index.min(item_count)
                    } else {
                        let relative = (cursor.x - bounds.origin.x).max(px(0.));
                        let slot_width = bounds.size.width / item_count as f32;
                        let slot = (relative / slot_width).floor() as usize;
                        let within_slot = relative - slot_width * slot as f32;
                        let insert_index = if within_slot > slot_width / 2. {
                            slot + 1
                        } else {
                            slot
                        };
                        insert_index.min(item_count)
                    };

                    state_for_container.update(cx, |s, cx| {
                        if s.drop_target_index != Some(target) {
                            s.drop_target_index = Some(target);
                            cx.notify();
                        }
                    });
                }
            })
            .on_drop({
                let state = state.clone();
                let on_reorder = on_reorder.clone();
                let on_insert = on_insert.clone();
                move |drag: &SortableDragData<T>, window, cx| {
                    let target = state.read(cx).drop_target_index.unwrap_or(item_count);
                    handle_drop(
                        drag,
                        target,
                        state_entity_id,
                        &state,
                        on_reorder.as_deref(),
                        on_insert.as_deref(),
                        window,
                        cx,
                    );
                }
            });

        let mut children: Vec<AnyElement> = Vec::with_capacity(items.len() * 2 + 1);

        for (index, item) in items.iter().enumerate() {
            // Indicator/spacer before this item
            let is_active = drop_target == Some(index)
                && !is_noop_target(dragging_from, drop_target);
            children.push(indicator(axis, gap, drag_border, is_active, index > 0).into_any_element());

            let is_dragging = dragging_from == Some(index);
            let item_id = (item_id_fn)(item);

            let drag_data = SortableDragData {
                item: item.clone(),
                from_index: index,
                source: state.clone(),
                render_item: render_fn.clone(),
            };

            let rendered = (render_fn)(item, index, window, cx);
            let state_for_drag = state.clone();
            let state_for_move = state.clone();

            let mut item_el = div()
                .id(item_id)
                .on_drag(drag_data, move |drag: &SortableDragData<T>, _pos, _window, cx| {
                    state_for_drag.update(cx, |s, _| {
                        s.dragging_from_index = Some(drag.from_index);
                    });
                    let entity = cx.new(|_| drag.clone());
                    let state_for_release = state_for_drag.clone();
                    let sub = cx.observe_release(&entity, move |_, cx| {
                        state_for_release.update(cx, |s, cx| {
                            s.dragging_from_index = None;
                            s.drag_item_size = None;
                            s.drop_target_index = None;
                            cx.notify();
                        });
                    });
                    state_for_drag.update(cx, |s, _| {
                        s._drag_subscription = Some(sub);
                    });
                    entity
                })
                .on_drag_move::<SortableDragData<T>>(
                    move |event: &DragMoveEvent<SortableDragData<T>>, _window, cx| {
                        state_for_move.update(cx, |s, _| {
                            s.drag_item_size = Some(event.bounds.size);
                        });
                    },
                );

            if is_dragging {
                item_el = item_el.child(
                    div()
                        .size_full()
                        .opacity(0.2)
                        .rounded_md()
                        .border_1()
                        .border_color(border_color)
                        .bg(ghost_bg)
                        .child(rendered),
                );
            } else {
                item_el = item_el.child(rendered);
            }

            children.push(item_el.into_any_element());
        }

        // Indicator after last item
        let is_active = drop_target == Some(item_count)
            && !is_noop_target(dragging_from, drop_target);
        children.push(
            indicator(axis, gap, drag_border, is_active, !items.is_empty()).into_any_element(),
        );

        container.children(children)
    }
}

fn clear_drag_state<T: Clone + 'static>(state: &Entity<SortableState<T>>, cx: &mut App) {
    state.update(cx, |s, cx| {
        clear_fields(s);
        cx.notify();
    });
}

fn clear_fields<T: Clone + 'static>(s: &mut SortableState<T>) {
    s.dragging_from_index = None;
    s.drag_item_size = None;
    s.drop_target_index = None;
}

fn is_noop_target(dragging_from: Option<usize>, drop_target: Option<usize>) -> bool {
    match (dragging_from, drop_target) {
        (Some(from), Some(to)) => to == from || to == from + 1,
        _ => false,
    }
}

fn spacer(axis: Axis, gap: Pixels) -> gpui::Div {
    div()
        .when(axis == Axis::Vertical, |d| d.h(gap))
        .when(axis == Axis::Horizontal, |d| d.w(gap))
}

fn indicator(
    axis: Axis,
    gap: Pixels,
    color: gpui::Hsla,
    active: bool,
    has_gap: bool,
) -> gpui::Div {
    if active {
        // Active indicator: colored line centered in the gap space
        div()
            .flex()
            .items_center()
            .justify_center()
            .when(axis == Axis::Vertical, |d| {
                d.w_full().h(gap).child(div().w_full().h(px(2.)).bg(color))
            })
            .when(axis == Axis::Horizontal, |d| {
                d.h_full().w(gap).child(div().h_full().w(px(2.)).bg(color))
            })
    } else if has_gap {
        spacer(axis, gap)
    } else {
        div()
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_drop<T: Clone + 'static>(
    drag: &SortableDragData<T>,
    zone_index: usize,
    this_list_id: gpui::EntityId,
    state: &Entity<SortableState<T>>,
    on_reorder: Option<&dyn Fn(usize, usize, &mut Window, &mut App)>,
    on_insert: Option<&dyn Fn(T, usize, Entity<SortableState<T>>, &mut Window, &mut App)>,
    window: &mut Window,
    cx: &mut App,
) {
    let from = drag.from_index;
    let same_list = drag.source.entity_id() == this_list_id;

    if same_list {
        if zone_index == from || zone_index == from + 1 || from >= state.read(cx).items.len() {
            clear_drag_state(state, cx);
            return;
        }

        let effective_to = if zone_index > from {
            zone_index - 1
        } else {
            zone_index
        };

        state.update(cx, |s, cx| {
            let item = s.items.remove(from);
            s.items.insert(effective_to, item);
            clear_fields(s);
            cx.notify();
        });

        if let Some(cb) = on_reorder {
            cb(from, effective_to, window, cx);
        }
    } else {
        let item = drag.item.clone();
        let source = drag.source.clone();

        source.update(cx, |s, cx| {
            if from < s.items.len() {
                s.items.remove(from);
            }
            clear_fields(s);
            cx.notify();
        });

        let insert_at = zone_index.min(state.read(cx).items.len());
        state.update(cx, |s, cx| {
            s.items.insert(insert_at, item.clone());
            clear_fields(s);
            cx.notify();
        });

        if let Some(cb) = on_insert {
            cb(item, insert_at, source, window, cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_noop_target() {
        assert!(is_noop_target(Some(2), Some(2)));
        assert!(is_noop_target(Some(2), Some(3)));
        assert!(!is_noop_target(Some(2), Some(0)));
        assert!(!is_noop_target(Some(2), Some(4)));
        assert!(!is_noop_target(None, Some(2)));
        assert!(!is_noop_target(Some(2), None));
        assert!(!is_noop_target(None, None));
    }

    #[test]
    fn test_clear_fields() {
        let mut state = SortableState::<String>::new(vec!["a".into(), "b".into()]);
        state.dragging_from_index = Some(1);
        state.drag_item_size = Some(Size {
            width: px(100.),
            height: px(50.),
        });
        state.drop_target_index = Some(0);

        clear_fields(&mut state);

        assert!(state.dragging_from_index.is_none());
        assert!(state.drag_item_size.is_none());
        assert!(state.drop_target_index.is_none());
        assert_eq!(state.items.len(), 2);
    }

    #[test]
    fn test_reorder_index_adjustment() {
        // Simulates the index adjustment logic from handle_drop for same-list reorder.
        // Moving item forward: zone_index > from → effective_to = zone_index - 1
        let from = 0;
        let zone_index = 3;
        let effective_to = if zone_index > from {
            zone_index - 1
        } else {
            zone_index
        };
        assert_eq!(effective_to, 2);

        // Moving item backward: zone_index <= from → effective_to = zone_index
        let from = 3;
        let zone_index = 1;
        let effective_to = if zone_index > from {
            zone_index - 1
        } else {
            zone_index
        };
        assert_eq!(effective_to, 1);
    }

    #[test]
    fn test_reorder_simulation() {
        let mut items = vec!["a", "b", "c", "d"];

        // Drag "a" (index 0) to after "c" (zone_index 3 → effective 2)
        let from = 0;
        let zone_index = 3;
        let effective_to = zone_index - 1; // zone_index > from
        let item = items.remove(from);
        items.insert(effective_to, item);
        assert_eq!(items, vec!["b", "c", "a", "d"]);

        // Drag "d" (index 3) to before "b" (zone_index 0 → effective 0)
        let from = 3;
        let zone_index = 0;
        let effective_to = zone_index; // zone_index <= from
        let item = items.remove(from);
        items.insert(effective_to, item);
        assert_eq!(items, vec!["d", "b", "c", "a"]);
    }

    #[test]
    fn test_cross_list_simulation() {
        let mut source = vec!["x", "y", "z"];
        let mut target = vec!["a", "b"];

        // Move "y" (index 1) from source to target at position 1
        let from = 1;
        let zone_index = 1;
        let item = source.remove(from);
        let insert_at = zone_index.min(target.len());
        target.insert(insert_at, item);

        assert_eq!(source, vec!["x", "z"]);
        assert_eq!(target, vec!["a", "y", "b"]);
    }

    #[test]
    fn test_state_builder() {
        let state = SortableState::new(vec![1, 2, 3]);
        assert_eq!(state.items(), &[1, 2, 3]);
        assert_eq!(state.len(), 3);
        assert!(!state.is_empty());

        let empty: SortableState<i32> = SortableState::new(vec![]);
        assert!(empty.is_empty());
    }
}
