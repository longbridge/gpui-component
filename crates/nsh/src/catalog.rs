//! The bridge from GPUI elements to NSH native components.
//!
//! A [`Control`] names an element NSH apps can use, declares its props and
//! events, and builds the element from them. The bridge owns everything
//! between NSH and that element: props arriving as JSON, events going back to
//! JS, the element's natural size, and the TypeScript that declares it. A
//! control's build function only calls the element's builder, so the
//! element's behavior stays in GPUI Kit, implemented once.

use std::{
    any::Any,
    cell::RefCell,
    collections::{HashMap, HashSet},
    fmt::Write as _,
    rc::Rc,
};

use gpui_kit::{
    AnyElement, App, AvailableSpace, Bounds, ElementId, FocusHandle, Hsla, IntoElement as _,
    ParentElement as _, Pixels, Refineable as _, SharedString, Size, StatefulInteractiveElement,
    StyleRefinement, Styled, Window,
    base::{StyledExt as _, Theme},
    inspector_reflection::FunctionReflection,
};
use nsh_platform_gpui::{ComponentContext, Components, NativeComponent};
use serde_json::{Map, Value};

/// The interaction-state class props of a control's root element
/// ([`Control::states`], [`Props::states`]).
pub const ROOT_STATES: [&str; 3] = ["hoverClass", "activeClass", "focusClass"];

/// The type of a prop or an event field, as TypeScript declares it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropType {
    /// `boolean`; a missing prop reads as `false`.
    Boolean,
    /// `number`; a missing prop reads as `0`.
    Number,
    /// `string`; a missing prop reads as empty.
    String,
    /// One of the listed strings; a missing prop reads as the first.
    Enum(&'static [&'static str]),
    /// `string[]`; a missing prop reads as empty.
    Strings,
}

/// A TypeScript object type of `fields`.
fn object(fields: &[String]) -> String {
    if fields.is_empty() {
        "{}".into()
    } else {
        format!("{{ {} }}", fields.join("; "))
    }
}

impl PropType {
    fn typescript(self) -> String {
        match self {
            PropType::Boolean => "boolean".into(),
            PropType::Number => "number".into(),
            PropType::String => "string".into(),
            PropType::Enum(values) => values
                .iter()
                .map(|value| format!("{value:?}"))
                .collect::<Vec<_>>()
                .join(" | "),
            PropType::Strings => "string[]".into(),
        }
    }
}

/// The retained state of a stateful control, made by its first build.
type State = Option<Box<dyn Any>>;
type Build = dyn Fn(&mut State, &Props, &Events, &mut Window, &mut App) -> AnyElement;
type Focus = dyn Fn(&dyn Any, &App) -> FocusHandle;

/// An element NSH apps can use as a native component.
pub struct Control {
    name: &'static str,
    documentation: &'static str,
    props: Vec<(&'static str, PropType)>,
    events: Vec<(&'static str, Vec<(&'static str, PropType)>)>,
    build: Rc<Build>,
    focus: Option<Rc<Focus>>,
}

impl Control {
    /// A control exported to TypeScript as `name`, drawn by `build`.
    pub fn new(
        name: &'static str,
        build: impl Fn(&Props, &Events, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self::with_build(
            name,
            Rc::new(move |_, props, events, window, cx| build(props, events, window, cx)),
        )
    }

    /// A control whose element needs state kept across frames (an entity,
    /// a subscription): `init` makes it once, `build` draws from it on
    /// every frame and brings it up to date with the props.
    pub fn stateful<S: 'static>(
        name: &'static str,
        init: impl Fn(&Props, &Events, &mut Window, &mut App) -> S + 'static,
        build: impl Fn(&mut S, &Props, &Events, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self::with_build(
            name,
            Rc::new(move |state: &mut State, props, events, window, cx| {
                let state = state.get_or_insert_with(|| Box::new(init(props, events, window, cx)));
                let state = state
                    .downcast_mut::<S>()
                    .expect("a control's state keeps its type");
                build(state, props, events, window, cx)
            }),
        )
    }

    /// A control whose element tracks a focus handle the control keeps, so
    /// it is a Tab stop among the app's text fields (GPUI Base's controls
    /// take one with `track_focus`).
    pub fn focusable(
        name: &'static str,
        build: impl Fn(&FocusHandle, &Props, &Events, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self::stateful(
            name,
            |_, _, _, cx| cx.focus_handle(),
            move |focus: &mut FocusHandle, props, events, window, cx| {
                build(focus, props, events, window, cx)
            },
        )
        .focus(|focus: &FocusHandle, _| focus.clone())
    }

    fn with_build(name: &'static str, build: Rc<Build>) -> Self {
        Self {
            name,
            documentation: "",
            props: Vec::new(),
            events: Vec::new(),
            build,
            focus: None,
        }
    }

    /// Makes a stateful control a Tab stop among the app's text fields,
    /// through the focus handle its state's widget tracks.
    pub fn focus<S: 'static>(mut self, focus: impl Fn(&S, &App) -> FocusHandle + 'static) -> Self {
        self.focus = Some(Rc::new(move |state, cx| {
            focus(
                state
                    .downcast_ref::<S>()
                    .expect("a control's state keeps its type"),
                cx,
            )
        }));
        self
    }

    /// The doc comment of its TypeScript declaration.
    pub fn documentation(mut self, documentation: &'static str) -> Self {
        self.documentation = documentation;
        self
    }

    /// Declares prop `name`, which JS may set (all props are optional).
    pub fn prop(mut self, name: &'static str, ty: PropType) -> Self {
        self.props.push((name, ty));
        self
    }

    /// Declares the class props of an interactive element's pointer and
    /// keyboard states, `[hover, active, focus]` (see [`Props::states`]). An
    /// empty name is a state the element does not have (a part that never
    /// takes focus).
    pub fn states(self, names: [&'static str; 3]) -> Self {
        names
            .into_iter()
            .filter(|name| !name.is_empty())
            .fold(self, |control, name| control.prop(name, PropType::String))
    }

    /// Declares event `name` (handled in JS as `on<Name>`) and its data.
    /// A field named after a prop also updates that prop at once, so the
    /// control shows the change before JS sets the prop back.
    pub fn event(mut self, name: &'static str, data: &[(&'static str, PropType)]) -> Self {
        self.events.push((name, data.to_vec()));
        self
    }

    /// The name TypeScript exports it as.
    pub fn name(&self) -> &'static str {
        self.name
    }
}

/// The props an app set on one control.
pub struct Props {
    id: ElementId,
    values: Rc<RefCell<Map<String, Value>>>,
    /// The props JS set since the last build.
    changed: HashSet<String>,
}

impl Props {
    /// An element id unique to this control instance.
    pub fn id(&self) -> ElementId {
        self.id.clone()
    }

    /// A [`PropType::Boolean`] prop.
    pub fn boolean(&self, name: &str) -> bool {
        self.value(name)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }

    /// A [`PropType::Number`] prop.
    pub fn number(&self, name: &str) -> f64 {
        self.value(name)
            .and_then(|value| value.as_f64())
            .unwrap_or(0.0)
    }

    /// A [`PropType::String`] or [`PropType::Enum`] prop, `None` when unset.
    pub fn string(&self, name: &str) -> Option<SharedString> {
        self.value(name)
            .and_then(|value| value.as_str().map(|s| SharedString::from(s.to_owned())))
    }

    /// A [`PropType::Strings`] prop.
    pub fn strings(&self, name: &str) -> Vec<SharedString> {
        match self.value(name) {
            Some(Value::Array(values)) => values
                .iter()
                .filter_map(|value| value.as_str().map(|s| SharedString::from(s.to_owned())))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Whether JS set prop `name` since the last frame. A stateful control
    /// pushes a prop into its state only then: an event's value updates the
    /// prop before the state's own change reaches JS, so comparing the prop
    /// with the state would undo an edit in progress.
    pub fn changed(&self, name: &str) -> bool {
        self.changed.contains(name)
    }

    /// Whether the app set prop `name`.
    pub fn has(&self, name: &str) -> bool {
        self.values.borrow().contains_key(name)
    }

    /// `element` with the styles of the `rootClass` prop. (NSH keeps `class`
    /// for the Tailwind classes of the box every element has.)
    pub fn styled<E: Styled>(&self, element: E, cx: &App) -> E {
        self.styled_by("rootClass", element, cx)
    }

    /// `element` with the class props of its interaction states, named
    /// `[hover, active, focus]`: GPUI Base styles semantic states (checked,
    /// selected, disabled), and the composer adds these. `focus` shows while
    /// the element has keyboard focus (`focus_visible`). None applies while
    /// the control is `disabled`.
    pub fn states<E: StatefulInteractiveElement>(
        &self,
        names: [&str; 3],
        element: E,
        cx: &App,
    ) -> E {
        self.state_styles(names, cx).apply(element)
    }

    /// The styles [`Props::states`] applies, for a part built later (a
    /// `'static` decorator).
    pub fn state_styles(&self, [hover, active, focus]: [&str; 3], cx: &App) -> StateStyles {
        // A disabled control answers neither the pointer nor the keyboard.
        if self.boolean("disabled") {
            return StateStyles::default();
        }
        let style = |name: &str| self.string(name).map(|class| class_style(&class, cx));
        StateStyles {
            hover: style(hover),
            active: style(active),
            focus: style(focus),
        }
    }

    /// `element` with the styles of prop `name`, a class string: GPUI's and
    /// GPUI Base's style methods that take no argument (`rounded_full p_1
    /// items_center`, found by reflection as GPUI Shell finds them), and
    /// `bg_<token>`, `text_<token>` and `border_<token>` for each color of
    /// GPUI Base's semantic tokens (`bg_primary`, `text_muted_foreground`).
    /// Unknown names are skipped with a warning.
    pub fn styled_by<E: Styled>(&self, name: &str, mut element: E, cx: &App) -> E {
        if let Some(class) = self.string(name) {
            element.style().refine(&class_style(&class, cx));
        }
        element
    }

    fn value(&self, name: &str) -> Option<Value> {
        self.values.borrow().get(name).cloned()
    }
}

/// The colors of GPUI Base's semantic tokens, by field name.
fn token_colors(cx: &App) -> HashMap<&'static str, Hsla> {
    let colors = Theme::global(cx).tokens.colors;
    macro_rules! tokens {
        ($($field:ident),* $(,)?) => {
            HashMap::from([$((stringify!($field), colors.$field)),*])
        };
    }
    tokens!(
        background,
        foreground,
        surface,
        surface_foreground,
        primary,
        primary_foreground,
        secondary,
        secondary_foreground,
        muted,
        muted_foreground,
        accent,
        accent_foreground,
        destructive,
        destructive_foreground,
        border,
        input,
        ring,
        selection,
    )
}

/// No-argument style methods reflection does not reach: GPUI Base makes
/// its font weights with a macro (GPUI Shell adds the same list).
type Nullary = fn(StyleRefinement) -> StyleRefinement;
const FONT_WEIGHTS: &[(&str, Nullary)] = &[
    ("font_thin", |style| style.font_thin()),
    ("font_extralight", |style| style.font_extralight()),
    ("font_light", |style| style.font_light()),
    ("font_normal", |style| style.font_normal()),
    ("font_medium", |style| style.font_medium()),
    ("font_semibold", |style| style.font_semibold()),
    ("font_bold", |style| style.font_bold()),
    ("font_extrabold", |style| style.font_extrabold()),
    ("font_black", |style| style.font_black()),
];

/// The style of a class string.
pub(crate) fn class_style(class: &str, cx: &App) -> StyleRefinement {
    let colors = token_colors(cx);
    thread_local! {
        static METHODS: HashMap<&'static str, FunctionReflection<StyleRefinement>> = [
            gpui_kit::base::styled_ext_reflection_methods::<StyleRefinement>(),
            gpui_kit::styled_reflection::methods::<StyleRefinement>(),
        ]
        .into_iter()
        .flatten()
        .map(|method| (method.name, method))
        .collect();
    }
    METHODS.with(|methods| {
        class
            .split_whitespace()
            .fold(StyleRefinement::default(), |style, name| {
                if let Some(method) = methods.get(name) {
                    return method.invoke(style);
                }
                if let Some((_, weight)) = FONT_WEIGHTS.iter().find(|(n, _)| *n == name) {
                    return weight(style);
                }
                // `opacity_<percent>`: GPUI's `opacity` takes an argument.
                if let Some(percent) = name
                    .strip_prefix("opacity_")
                    .and_then(|percent| percent.parse::<u8>().ok())
                    .filter(|percent| *percent <= 100)
                {
                    return style.opacity(f32::from(percent) / 100.0);
                }
                let token = |prefix: &str| {
                    name.strip_prefix(prefix)
                        .and_then(|token| colors.get(token).copied())
                };
                if let Some(color) = token("bg_") {
                    style.bg(color)
                } else if let Some(color) = token("text_") {
                    style.text_color(color)
                } else if let Some(color) = token("border_") {
                    style.border_color(color)
                } else {
                    log::warn!("`{name}` is not a style method that takes no argument");
                    style
                }
            })
    })
}

/// Where a control's events go: the NSH app, or a test's record.
pub(crate) trait Sink {
    fn emit(&self, name: &str, data: Value);
    fn redraw(&self);
}

impl Sink for ComponentContext {
    fn emit(&self, name: &str, data: Value) {
        ComponentContext::emit(self, name, data);
    }

    fn redraw(&self) {
        ComponentContext::redraw(self);
    }
}

/// The interaction-state styles of an element ([`Props::state_styles`]).
#[derive(Clone, Default)]
pub struct StateStyles {
    hover: Option<StyleRefinement>,
    active: Option<StyleRefinement>,
    focus: Option<StyleRefinement>,
}

impl StateStyles {
    /// `element` with these styles while hovered, pressed and focused from
    /// the keyboard.
    pub fn apply<E: StatefulInteractiveElement>(self, mut element: E) -> E {
        if let Some(style) = self.hover {
            element = element.hover(|_| style);
        }
        if let Some(style) = self.active {
            element = element.active(|_| style);
        }
        if let Some(style) = self.focus {
            element = element.focus_visible(|_| style);
        }
        element
    }
}

/// Sends a control's events to JS.
#[derive(Clone)]
pub struct Events {
    sink: Rc<dyn Sink>,
    values: Rc<RefCell<Map<String, Value>>>,
    props: Rc<[&'static str]>,
}

impl Events {
    /// Emits `name` with `data` (a JSON object).
    pub fn emit(&self, name: &str, data: Value) {
        if let Value::Object(fields) = &data {
            let mut values = self.values.borrow_mut();
            for (field, value) in fields {
                if self.props.contains(&field.as_str()) {
                    values.insert(field.clone(), value.clone());
                }
            }
        }
        self.sink.emit(name, data);
        self.sink.redraw();
    }

    /// A GPUI handler emitting `name` with the data `data` makes of its value.
    pub fn handler<T: ?Sized>(
        &self,
        name: &'static str,
        data: impl Fn(&T) -> Value + 'static,
    ) -> impl Fn(&T, &mut Window, &mut App) + 'static {
        let events = self.clone();
        move |value, _, _| events.emit(name, data(value))
    }
}

/// A set of controls registered with NSH views under one prefix, as
/// `"<prefix>.<Name>"`.
pub struct Catalog {
    prefix: &'static str,
    controls: Vec<Rc<Control>>,
}

impl Catalog {
    /// An empty catalog whose controls are registered as `"<prefix>.<Name>"`.
    pub fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            controls: Vec::new(),
        }
    }

    /// The controls, in the order they were added.
    pub fn controls(&self) -> impl Iterator<Item = &Control> {
        self.controls.iter().map(|control| control.as_ref())
    }

    #[cfg(test)]
    pub(crate) fn find(&self, name: &str) -> Option<Rc<Control>> {
        self.controls.iter().find(|c| c.name == name).cloned()
    }

    /// Adds `control`, replacing one of the same name.
    pub fn control(mut self, control: Control) -> Self {
        self.controls.retain(|c| c.name != control.name);
        self.controls.push(Rc::new(control));
        self
    }

    /// Registers every control in `components`, the native components of
    /// the views made with them (`AppOptions::components`).
    pub fn register(&self, components: &Components) {
        for control in &self.controls {
            let control = control.clone();
            components.register(
                &format!("{}.{}", self.prefix, control.name),
                Rc::new(move |context| Box::new(Instance::new(control.clone(), context))),
            );
        }
    }

    /// The TypeScript module declaring every control for NSH apps, which
    /// they import in place of hand-written `nativeComponent` calls.
    pub fn typescript(&self) -> String {
        let mut out = String::from(
            "// Generated by gpui-nsh from its catalog. Do not edit.\n\n\
             import { nativeComponent } from \"nsh/native\";\n",
        );
        for control in &self.controls {
            let props = control
                .props
                .iter()
                .map(|(name, ty)| format!("{name}?: {}", ty.typescript()))
                .collect::<Vec<_>>();
            let events = control
                .events
                .iter()
                .map(|(name, data)| {
                    let fields = data
                        .iter()
                        .map(|(field, ty)| format!("{field}: {}", ty.typescript()))
                        .collect::<Vec<_>>();
                    format!("{name}: {}", object(&fields))
                })
                .collect::<Vec<_>>();
            let quoted = |names: Vec<&str>| {
                names
                    .iter()
                    .map(|name| format!("{name:?}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            out.push('\n');
            if !control.documentation.is_empty() {
                let _ = writeln!(out, "/** {} */", control.documentation);
            }
            let _ = writeln!(
                out,
                "export const {name} = nativeComponent<{props}, {events}>(\"{prefix}.{name}\", {{\n  \
                 props: [{prop_names}],\n  events: [{event_names}],\n}});",
                name = control.name,
                prefix = self.prefix,
                props = object(&props),
                events = object(&events),
                prop_names = quoted(control.props.iter().map(|(name, _)| *name).collect()),
                event_names = quoted(control.events.iter().map(|(name, _)| *name).collect()),
            );
        }
        out
    }
}

/// One control with its props, events and state: what the NSH component
/// and the tests drive.
pub(crate) struct Mounted {
    control: Rc<Control>,
    props: Props,
    events: Events,
    state: State,
    /// The focus handle of a [`Control::focus`] control, as of the last frame.
    focus: Option<FocusHandle>,
}

impl Mounted {
    pub(crate) fn new(control: Rc<Control>, sink: Rc<dyn Sink>) -> Self {
        let values = Rc::new(RefCell::new(Map::new()));
        let id = ElementId::NamedInteger(control.name.into(), next_instance());
        let events = Events {
            sink,
            values: values.clone(),
            props: control.props.iter().map(|(name, _)| *name).collect(),
        };
        Self {
            control,
            props: Props {
                id,
                values,
                changed: HashSet::new(),
            },
            events,
            state: None,
            focus: None,
        }
    }

    pub(crate) fn set_prop(&mut self, name: &str, value: Option<&Value>) {
        self.props.changed.insert(name.to_owned());
        let mut values = self.props.values.borrow_mut();
        match value {
            Some(value) => values.insert(name.to_owned(), value.clone()),
            None => values.remove(name),
        };
    }

    #[cfg(test)]
    pub(crate) fn id(&self) -> ElementId {
        self.props.id()
    }

    pub(crate) fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }

    /// The element for this frame. Its text takes GPUI Base's foreground
    /// until the control's own style says otherwise.
    pub(crate) fn build(&mut self, window: &mut Window, cx: &mut App) -> AnyElement {
        let element = (self.control.build)(&mut self.state, &self.props, &self.events, window, cx);
        self.props.changed.clear();
        // Asked on every frame: a control may replace the entity it focuses,
        // and a disabled control is no Tab stop.
        self.focus = match (&self.control.focus, &self.state) {
            (Some(focus), Some(state)) if !self.props.boolean("disabled") => {
                Some(focus(state.as_ref(), cx))
            }
            _ => None,
        };
        let foreground = Theme::global(cx).tokens.colors.foreground;
        gpui_kit::div()
            .text_color(foreground)
            .child(element)
            .into_any_element()
    }

    /// The element's natural size: at its max-content width (labels on one
    /// line), or at `width` once the box has one (wrapped text is then as
    /// tall as its lines).
    pub(crate) fn measure(
        &mut self,
        width: Option<Pixels>,
        window: &mut Window,
        cx: &mut App,
    ) -> Size<Pixels> {
        self.build(window, cx).layout_as_root(
            Size {
                width: width.map_or(AvailableSpace::MaxContent, AvailableSpace::Definite),
                height: AvailableSpace::MinContent,
            },
            window,
            cx,
        )
    }
}

/// One control in an NSH app.
struct Instance {
    mounted: Mounted,
    context: ComponentContext,
    /// Set when the props or the box's width changed: the natural size is
    /// measured again.
    measure: bool,
    /// The box's width when something other than the control's own size
    /// decides it (the app's style, a flex parent); `None` until then.
    width: Option<Pixels>,
    /// The width last reported to NSH.
    reported: Option<f32>,
}

impl Instance {
    fn new(control: Rc<Control>, context: ComponentContext) -> Self {
        Self {
            mounted: Mounted::new(control, Rc::new(context.clone())),
            context,
            measure: true,
            width: None,
            reported: None,
        }
    }
}

/// A process-wide counter for element ids.
fn next_instance() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

impl NativeComponent for Instance {
    fn render(&mut self, window: &mut Window, cx: &mut App) -> Option<AnyElement> {
        if std::mem::take(&mut self.measure) {
            // The element's natural size, where the app's style leaves the
            // box's size open.
            let size = self.mounted.measure(self.width, window, cx);
            let width = f32::from(size.width).ceil();
            self.reported = Some(width);
            self.context.set_size(width, f32::from(size.height).ceil());
        }
        Some(self.mounted.build(window, cx))
    }

    fn placed(&mut self, bounds: Option<Bounds<Pixels>>) {
        // A box drawn at another width than the one reported has a width of
        // its own: the height is measured at it, so wrapped text fits. A box
        // not laid out yet (zero wide, or at the reported width, since NSH
        // applies sizes asynchronously) changes nothing.
        let Some(width) = bounds.map(|bounds| bounds.size.width) else {
            return;
        };
        let own = width > Pixels::ZERO
            && self
                .reported
                .is_some_and(|reported| (f32::from(width) - reported).abs() >= 1.0);
        if own && self.width != Some(width) {
            self.width = Some(width);
            self.measure = true;
            self.context.redraw();
        }
    }

    fn focus_handle(&self) -> Option<FocusHandle> {
        self.mounted.focus_handle()
    }

    fn set_prop(&mut self, name: &str, value: Option<&Value>, _: &mut Window, _: &mut App) {
        self.mounted.set_prop(name, value);
        self.measure = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Silent;

    impl Sink for Silent {
        fn emit(&self, _: &str, _: Value) {}
        fn redraw(&self) {}
    }

    #[test]
    fn test_props_changed_counts_only_js() {
        let control = Rc::new(
            Control::new("Field", |_, _, _, _| unreachable!()).prop("value", PropType::String),
        );
        let mut mounted = Mounted::new(control, Rc::new(Silent));
        mounted.set_prop("value", Some(&serde_json::json!("a")));
        assert!(mounted.props.changed("value"));
        mounted.props.changed.clear();

        // An event's value updates the prop at once, but is not JS's change.
        mounted
            .events
            .emit("change", serde_json::json!({ "value": "ab" }));
        assert_eq!(mounted.props.string("value").as_deref(), Some("ab"));
        assert!(!mounted.props.changed("value"));
    }

    #[gpui_kit::test]
    fn test_state_styles(cx: &mut gpui_kit::TestAppContext) {
        let control = Control::new("Item", |_, _, _, _| unreachable!()).states([
            "itemHoverClass",
            "itemActiveClass",
            "",
        ]);
        let declared: Vec<_> = control.props.iter().map(|(name, _)| *name).collect();
        assert_eq!(declared, ["itemHoverClass", "itemActiveClass"]);

        let mut mounted = Mounted::new(Rc::new(control), Rc::new(Silent));
        mounted.set_prop("itemHoverClass", Some(&serde_json::json!("bg_accent")));
        mounted.set_prop("itemActiveClass", Some(&serde_json::json!("bg_primary")));
        cx.update(|cx| {
            gpui_kit::init(cx);
            let names = ["itemHoverClass", "itemActiveClass", ""];
            let styles = mounted.props.state_styles(names, cx);
            assert!(styles.hover.is_some() && styles.active.is_some());
            assert!(styles.focus.is_none());

            // A disabled control shows no interaction states.
            mounted.set_prop("disabled", Some(&serde_json::json!(true)));
            let styles = mounted.props.state_styles(names, cx);
            assert!(styles.hover.is_none() && styles.active.is_none());
        });
    }

    #[gpui_kit::test]
    fn test_class_style(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            let primary = Theme::global(cx).tokens.colors.primary;
            let style = class_style("flex items_center w_9 rounded_full bg_primary unknown", cx);
            assert_eq!(style.display, Some(gpui_kit::Display::Flex));
            assert_eq!(style.align_items, Some(gpui_kit::AlignItems::Center));
            assert!(style.size.width.is_some());
            assert!(style.corner_radii.top_left.is_some());
            assert_eq!(style.background, Some(primary.into()));
            let style = class_style("font_semibold opacity_50 opacity_101", cx);
            assert_eq!(style.text.font_weight, Some(gpui_kit::FontWeight::SEMIBOLD));
            assert_eq!(style.opacity, Some(0.5));
            let style = class_style("text_muted_foreground border_border", cx);
            assert!(style.text.color.is_some());
            assert!(style.border_color.is_some());
        });
    }

    #[test]
    fn test_catalog_typescript() {
        let catalog = Catalog::new("demo").control(
            Control::new("Toggle", |_, _, _, _| unreachable!())
                .documentation("A toggle.")
                .prop("checked", PropType::Boolean)
                .prop("size", PropType::Enum(&["small", "large"]))
                .event("change", &[("checked", PropType::Boolean)]),
        );
        assert_eq!(
            catalog.typescript(),
            "// Generated by gpui-nsh from its catalog. Do not edit.\n\n\
             import { nativeComponent } from \"nsh/native\";\n\n\
             /** A toggle. */\n\
             export const Toggle = nativeComponent<{ checked?: boolean; size?: \"small\" | \"large\" }, \
             { change: { checked: boolean } }>(\"demo.Toggle\", {\n  \
             props: [\"checked\", \"size\"],\n  events: [\"change\"],\n});\n"
        );
    }
}
