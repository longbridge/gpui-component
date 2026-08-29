use gpui_shell::{
    ArgumentDescriptor, ArgumentSchema, ComponentArgument, ComponentCallbackArgument,
    ComponentDescriptor, ComponentMaterializer, ComponentPayload, ComponentRegistry,
    ConstructorDescriptor, MaterializeRequest, MethodDescriptor, RegistryError,
    TypeScriptDescriptor, anyhow,
    gpui::{
        self, Axis, Bounds, Element, ElementId, GlobalElementId, InspectorElementId,
        IntoElement as _, LayoutId, Pixels, Refineable as _, Window,
    },
    gpui_component::{
        Selectable as _, Sizable as _, Size,
        accordion::{Accordion, AccordionItem},
        radio::{Radio, RadioGroup},
        stepper::{Stepper, StepperItem},
        tab::{Tab, TabBar, TabVariant},
    },
};
use std::sync::Arc;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_registered_child_identity_is_rejected() {
        assert!(require_child("Accordion", "AccordionItem", Some("Tab")).is_err());
        let nested = require_child("Accordion", "AccordionItem", None).unwrap_err();
        assert!(nested.to_string().contains("ordinary element"), "{nested}");
        assert!(require_child("RadioGroup", "Radio", None).is_err());
        assert!(require_child("Stepper", "StepperItem", Some("StepperItem")).is_ok());
    }

    #[test]
    fn callback_operation_preserves_the_script_callback_handle() {
        let payload = index_callback_payload(
            &[ComponentArgument::Callback(42)],
            "RadioGroup",
            RadioGroupOp::OnChange,
        )
        .unwrap();
        assert!(matches!(
            payload.downcast_ref::<RadioGroupOp>(),
            Some(RadioGroupOp::OnChange(ComponentArgument::Callback(42)))
        ));
        assert!(
            index_callback_payload(
                &[ComponentArgument::Number(42.0)],
                "RadioGroup",
                RadioGroupOp::OnChange,
            )
            .is_err()
        );
    }

    #[test]
    fn typed_elements_are_extracted_from_real_any_elements() {
        let mut accordion_item = TypedChildElement::new(AccordionItem::new()).into_any_element();
        let mut radio = TypedChildElement::new(Radio::new("radio")).into_any_element();
        let mut tab = TypedChildElement::new(Tab::new()).into_any_element();
        let mut stepper_item = TypedChildElement::new(StepperItem::new()).into_any_element();

        take_element::<AccordionItem>(&mut accordion_item, "AccordionItem").unwrap();
        take_element::<Radio>(&mut radio, "Radio").unwrap();
        take_element::<Tab>(&mut tab, "Tab").unwrap();
        take_element::<StepperItem>(&mut stepper_item, "StepperItem").unwrap();

        assert!(take_element::<StepperItem>(&mut tab, "StepperItem").is_err());
    }

    #[test]
    fn batch_publishes_closed_documented_descriptors() {
        let mut registry =
            ComponentRegistry::new(gpui_shell::COMPONENT_REGISTRY_API_VERSION).unwrap();
        register(&mut registry).unwrap();
        let frozen = registry.freeze().unwrap();
        let names = frozen
            .descriptors()
            .map(|descriptor| descriptor.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            [
                "AccordionItem",
                "Accordion",
                "RadioGroup",
                "Tab",
                "TabBar",
                "StepperItem",
                "Stepper",
            ]
        );
        assert!(frozen.descriptors().all(|descriptor| {
            descriptor.typescript.documentation.is_some()
                && descriptor
                    .methods
                    .iter()
                    .all(|method| method.documentation.is_some())
        }));
        let tab_bar = frozen
            .descriptors()
            .find(|descriptor| descriptor.name == "TabBar")
            .unwrap();
        assert_eq!(
            tab_bar.constructors[0].arguments[0].schema,
            ArgumentSchema::String
        );
        assert_eq!(
            tab_bar.methods[1].arguments[0].schema,
            ArgumentSchema::Enum(&["tab", "outline", "pill", "segmented", "underline"])
        );
    }
}

fn require_child(
    parent: &str,
    expected: &'static str,
    actual: Option<&'static str>,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        actual == Some(expected),
        "{parent} accepts only registered {expected} children; received {}",
        actual.unwrap_or("an ordinary element")
    );
    Ok(())
}

fn take_element<T: gpui::IntoElement + 'static>(
    element: &mut gpui::AnyElement,
    name: &str,
) -> anyhow::Result<T> {
    element
        .downcast_mut::<TypedChildElement<T>>()
        .ok_or_else(|| anyhow::anyhow!("registered {name} materialized an incompatible element"))?
        .take()
        .ok_or_else(|| anyhow::anyhow!("registered {name} child was already consumed"))
}

struct TypedChildElement<T: gpui::IntoElement + 'static> {
    value: Option<T>,
}

impl<T: gpui::IntoElement + 'static> TypedChildElement<T> {
    fn new(value: T) -> Self {
        Self { value: Some(value) }
    }

    fn take(&mut self) -> Option<T> {
        self.value.take()
    }
}

impl<T: gpui::IntoElement + 'static> gpui::IntoElement for TypedChildElement<T> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl<T: gpui::IntoElement + 'static> Element for TypedChildElement<T> {
    type RequestLayoutState = gpui::AnyElement;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut gpui::App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut element = self
            .take()
            .expect("typed child element can request layout only once")
            .into_any_element();
        let layout = element.request_layout(window, cx);
        (layout, element)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        element: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        element.paint(window, cx);
    }
}

pub(super) fn finish_part<E>(
    request: &mut MaterializeRequest<'_>,
    mut element: E,
) -> anyhow::Result<gpui::AnyElement>
where
    E: gpui::Styled + gpui::ParentElement + gpui::IntoElement + 'static,
{
    element.style().refine(&request.take_style());
    element.extend(request.take_children()?);
    Ok(TypedChildElement::new(element).into_any_element())
}

fn finish_typed<E>(request: &mut MaterializeRequest<'_>, mut element: E) -> gpui::AnyElement
where
    E: gpui::Styled + gpui::IntoElement + 'static,
{
    element.style().refine(&request.take_style());
    element.into_any_element()
}

fn string_payload(
    arguments: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(String) -> ComponentPayload,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::String(value)] => Ok(make(value.clone())),
        _ => Err(format!("{callable} expects one string")),
    }
}

fn bool_payload<T: Send + Sync + 'static>(
    arguments: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(bool) -> T,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::Boolean(value)] => Ok(ComponentPayload::new(make(*value))),
        _ => Err(format!("{callable} expects one boolean")),
    }
}

fn index_payload<T: Send + Sync + 'static>(
    arguments: &[ComponentArgument],
    callable: &str,
    make: impl FnOnce(usize) -> T,
) -> Result<ComponentPayload, String> {
    match arguments {
        [ComponentArgument::Number(value)]
            if value.is_finite()
                && *value >= 0.0
                && value.fract() == 0.0
                && *value <= usize::MAX as f64 =>
        {
            Ok(ComponentPayload::new(make(*value as usize)))
        }
        _ => Err(format!("{callable} expects a nonnegative integer")),
    }
}

fn id_constructor<T: Send + Sync + 'static>(
    component: &'static str,
    make: impl Fn(String) -> T + Send + Sync + 'static,
) -> ConstructorDescriptor {
    ConstructorDescriptor::new(
        component,
        vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
        move |arguments| match arguments {
            [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                Ok(ComponentPayload::new(make(id.clone())))
            }
            _ => Err(format!("{component}(id) expects a nonempty string id")),
        },
    )
}

fn size(value: &str) -> Result<Size, String> {
    match value {
        "xsmall" => Ok(Size::XSmall),
        "small" => Ok(Size::Small),
        "medium" => Ok(Size::Medium),
        "large" => Ok(Size::Large),
        _ => Err(format!("unsupported semantic size `{value}`")),
    }
}

fn size_method<T: Send + Sync + 'static>(
    component: &'static str,
    make: impl Fn(Size) -> T + Send + Sync + 'static,
) -> MethodDescriptor {
    MethodDescriptor::new(
        "size",
        vec![ArgumentDescriptor::new(
            "size",
            ArgumentSchema::Enum(&["xsmall", "small", "medium", "large"]),
        )],
        move |arguments| match arguments {
            [ComponentArgument::Enum(value)] => {
                size(value).map(|value| ComponentPayload::new(make(value)))
            }
            _ => Err(format!("{component}.size(size) expects a semantic size")),
        },
    )
    .documented("Sets the semantic component size.")
}

#[derive(Clone, Copy)]
struct AccordionItemPayload;

#[derive(Clone)]
enum AccordionItemOp {
    Title(ComponentArgument),
    Open(bool),
    Disabled(bool),
}

struct AccordionItemMaterializer;

impl ComponentMaterializer for AccordionItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<AccordionItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("AccordionItem received an incompatible payload"))?;
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<AccordionItemOp>().cloned())
            .collect::<Vec<_>>();
        let mut item = AccordionItem::new().disabled(request.disabled());
        for operation in operations {
            item = match operation {
                AccordionItemOp::Title(argument) => item.title(request.resolve_element(&argument)?),
                AccordionItemOp::Open(value) => item.open(value),
                AccordionItemOp::Disabled(value) => item.disabled(value),
            };
        }
        finish_part(&mut request, item)
    }
}

#[derive(Clone)]
struct AccordionPayload(String);

#[derive(Clone, Copy)]
enum AccordionOp {
    Multiple(bool),
    Bordered(bool),
    Size(Size),
}

struct AccordionMaterializer;

impl ComponentMaterializer for AccordionMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<AccordionPayload>()
            .ok_or_else(|| anyhow::anyhow!("Accordion received an incompatible payload"))?
            .0
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<AccordionOp>().copied())
            .collect::<Vec<_>>();
        let mut children = request.take_typed_children();
        for child in &children {
            require_child("Accordion", "AccordionItem", child.component_name())?;
        }
        let mut accordion = Accordion::new(id).disabled(request.disabled());
        for operation in operations {
            accordion = match operation {
                AccordionOp::Multiple(value) => accordion.multiple(value),
                AccordionOp::Bordered(value) => accordion.bordered(value),
                AccordionOp::Size(value) => accordion.with_size(value),
            };
        }
        for child in &mut children {
            let mut element = request.materialize_child(child)?;
            let item = take_element::<AccordionItem>(&mut element, "AccordionItem")?;
            accordion = accordion.item(|_| item);
        }
        Ok(finish_typed(&mut request, accordion))
    }
}

#[derive(Clone)]
struct RadioGroupPayload {
    id: String,
    axis: Axis,
}

#[derive(Clone)]
enum RadioGroupOp {
    Selected(usize),
    Disabled(bool),
    OnChange(ComponentArgument),
}

struct RadioGroupMaterializer;

impl ComponentMaterializer for RadioGroupMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let payload = request
            .payload()
            .downcast_ref::<RadioGroupPayload>()
            .ok_or_else(|| anyhow::anyhow!("RadioGroup received an incompatible payload"))?
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<RadioGroupOp>().cloned())
            .collect::<Vec<_>>();
        let mut children = request.take_typed_children();
        for child in &children {
            require_child("RadioGroup", "Radio", child.component_name())?;
        }
        let mut group = match payload.axis {
            Axis::Horizontal => RadioGroup::horizontal(payload.id),
            Axis::Vertical => RadioGroup::vertical(payload.id),
        }
        .disabled(request.disabled());
        for operation in operations {
            group = match operation {
                RadioGroupOp::Selected(value) => group.selected_index(Some(value)),
                RadioGroupOp::Disabled(value) => group.disabled(value),
                RadioGroupOp::OnChange(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    group.on_click(move |index, window, cx| {
                        callback.invoke_and_report_with(
                            "RadioGroup.onChange callback failed",
                            &[ComponentCallbackArgument::Number(*index as f64)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        for child in &mut children {
            let mut element = request.materialize_child(child)?;
            group = group.child(take_element::<Radio>(&mut element, "Radio")?);
        }
        Ok(finish_typed(&mut request, group))
    }
}

#[derive(Clone, Copy)]
struct TabPayload;

#[derive(Clone)]
enum TabOp {
    Label(String),
    AriaLabel(String),
    Disabled(bool),
    Selected(bool),
}

struct TabMaterializer;

impl ComponentMaterializer for TabMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<TabPayload>()
            .ok_or_else(|| anyhow::anyhow!("Tab received an incompatible payload"))?;
        let mut tab = Tab::new()
            .disabled(request.disabled())
            .selected(request.selected());
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<TabOp>())
        {
            tab = match operation {
                TabOp::Label(value) => tab.label(value.clone()),
                TabOp::AriaLabel(value) => tab.aria_label(value.clone()),
                TabOp::Disabled(value) => tab.disabled(*value),
                TabOp::Selected(value) => tab.selected(*value),
            };
        }
        finish_part(&mut request, tab)
    }
}

#[derive(Clone)]
struct TabBarPayload(String);

#[derive(Clone)]
enum TabBarOp {
    Selected(usize),
    Variant(TabVariant),
    Menu(bool),
    Size(Size),
    OnChange(ComponentArgument),
}

struct TabBarMaterializer;

impl ComponentMaterializer for TabBarMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<TabBarPayload>()
            .ok_or_else(|| anyhow::anyhow!("TabBar received an incompatible payload"))?
            .0
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<TabBarOp>().cloned())
            .collect::<Vec<_>>();
        let mut children = request.take_typed_children();
        for child in &children {
            require_child("TabBar", "Tab", child.component_name())?;
        }
        let mut bar = TabBar::new(id);
        for operation in operations {
            bar = match operation {
                TabBarOp::Selected(value) => bar.selected_index(value),
                TabBarOp::Variant(value) => bar.with_variant(value),
                TabBarOp::Menu(value) => bar.menu(value),
                TabBarOp::Size(value) => bar.with_size(value),
                TabBarOp::OnChange(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    bar.on_click(move |index, window, cx| {
                        callback.invoke_and_report_with(
                            "TabBar.onChange callback failed",
                            &[ComponentCallbackArgument::Number(*index as f64)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        for child in &mut children {
            let mut element = request.materialize_child(child)?;
            bar = bar.child(take_element::<Tab>(&mut element, "Tab")?);
        }
        Ok(finish_typed(&mut request, bar))
    }
}

#[derive(Clone, Copy)]
struct StepperItemPayload;

#[derive(Clone, Copy)]
enum StepperItemOp {
    Disabled(bool),
}

struct StepperItemMaterializer;

impl ComponentMaterializer for StepperItemMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        request
            .payload()
            .downcast_ref::<StepperItemPayload>()
            .ok_or_else(|| anyhow::anyhow!("StepperItem received an incompatible payload"))?;
        let mut item = StepperItem::new().disabled(request.disabled());
        for operation in request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<StepperItemOp>())
        {
            match operation {
                StepperItemOp::Disabled(value) => item = item.disabled(*value),
            }
        }
        finish_part(&mut request, item)
    }
}

#[derive(Clone)]
struct StepperPayload(String);

#[derive(Clone)]
enum StepperOp {
    Selected(usize),
    Vertical(bool),
    TextCenter(bool),
    Disabled(bool),
    Size(Size),
    OnChange(ComponentArgument),
}

struct StepperMaterializer;

impl ComponentMaterializer for StepperMaterializer {
    fn materialize(&self, mut request: MaterializeRequest<'_>) -> anyhow::Result<gpui::AnyElement> {
        let id = request
            .payload()
            .downcast_ref::<StepperPayload>()
            .ok_or_else(|| anyhow::anyhow!("Stepper received an incompatible payload"))?
            .0
            .clone();
        let operations = request
            .methods()
            .filter_map(|method| method.payload().downcast_ref::<StepperOp>().cloned())
            .collect::<Vec<_>>();
        let mut children = request.take_typed_children();
        for child in &children {
            require_child("Stepper", "StepperItem", child.component_name())?;
        }
        let mut stepper = Stepper::new(id).disabled(request.disabled());
        for operation in operations {
            stepper = match operation {
                StepperOp::Selected(value) => stepper.selected_index(value),
                StepperOp::Vertical(value) => stepper.layout(if value {
                    Axis::Vertical
                } else {
                    Axis::Horizontal
                }),
                StepperOp::TextCenter(value) => stepper.text_center(value),
                StepperOp::Disabled(value) => stepper.disabled(value),
                StepperOp::Size(value) => stepper.with_size(value),
                StepperOp::OnChange(argument) => {
                    let callback = request.resolve_callback(&argument)?;
                    stepper.on_click(move |index, window, cx| {
                        callback.invoke_and_report_with(
                            "Stepper.onChange callback failed",
                            &[ComponentCallbackArgument::Number(*index as f64)],
                            window,
                            cx,
                        );
                    })
                }
            };
        }
        for child in &mut children {
            let mut element = request.materialize_child(child)?;
            stepper = stepper.item(take_element::<StepperItem>(&mut element, "StepperItem")?);
        }
        Ok(finish_typed(&mut request, stepper))
    }
}

fn bool_method<T: Send + Sync + 'static>(
    component: &'static str,
    name: &'static str,
    documentation: &'static str,
    make: impl Fn(bool) -> T + Send + Sync + 'static,
) -> MethodDescriptor {
    MethodDescriptor::new(
        name,
        vec![ArgumentDescriptor::new(name, ArgumentSchema::Boolean)],
        move |arguments| bool_payload(arguments, &format!("{component}.{name}({name})"), &make),
    )
    .documented(documentation)
}

fn index_callback_method<T: Send + Sync + 'static>(
    component: &'static str,
    make: impl Fn(ComponentArgument) -> T + Send + Sync + 'static,
) -> MethodDescriptor {
    MethodDescriptor::new(
        "onChange",
        vec![ArgumentDescriptor::new(
            "callback",
            ArgumentSchema::Callback("(index: number) => void"),
        )],
        move |arguments| index_callback_payload(arguments, component, &make),
    )
    .documented("Reports the newly selected zero-based index.")
}

fn index_callback_payload<T: Send + Sync + 'static>(
    arguments: &[ComponentArgument],
    component: &'static str,
    make: impl Fn(ComponentArgument) -> T,
) -> Result<ComponentPayload, String> {
    match arguments {
        [value @ ComponentArgument::Callback(_)] => Ok(ComponentPayload::new(make(value.clone()))),
        _ => Err(format!("{component}.onChange(callback) expects a callback")),
    }
}

pub(crate) fn register(registry: &mut ComponentRegistry) -> Result<(), RegistryError> {
    registry.register(ComponentDescriptor {
        name: "AccordionItem",
        constructors: vec![ConstructorDescriptor::new("AccordionItem", vec![], |_| {
            Ok(ComponentPayload::new(AccordionItemPayload))
        })],
        methods: vec![
            MethodDescriptor::new(
                "title",
                vec![ArgumentDescriptor::new("title", ArgumentSchema::Element)],
                |arguments| match arguments {
                    [value @ ComponentArgument::Element(_)] => {
                        Ok(ComponentPayload::new(AccordionItemOp::Title(value.clone())))
                    }
                    _ => Err("AccordionItem.title(title) expects an element".into()),
                },
            )
            .documented("Sets the interactive title row element."),
            bool_method(
                "AccordionItem",
                "open",
                "Controls expanded state.",
                AccordionItemOp::Open,
            ),
            bool_method(
                "AccordionItem",
                "disabled",
                "Disables this item.",
                AccordionItemOp::Disabled,
            ),
        ],
        typescript: TypeScriptDescriptor::new(
            "An accordion part accepted only as a direct Accordion child.",
        ),
        materializer: Arc::new(AccordionItemMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "Accordion",
        constructors: vec![id_constructor("Accordion", AccordionPayload)],
        methods: vec![
            bool_method(
                "Accordion",
                "multiple",
                "Allows multiple items to remain open.",
                AccordionOp::Multiple,
            ),
            bool_method(
                "Accordion",
                "bordered",
                "Controls the joined outer border.",
                AccordionOp::Bordered,
            ),
            size_method("Accordion", AccordionOp::Size),
        ],
        typescript: TypeScriptDescriptor::new(
            "A typed accordion container accepting only AccordionItem children. Toggle callbacks are not exposed: the real component requires a Send + Sync handler, while shell callbacks are runtime-local.",
        ),
        materializer: Arc::new(AccordionMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "RadioGroup",
        constructors: vec![
            id_constructor("RadioGroup", |id| RadioGroupPayload {
                id,
                axis: Axis::Vertical,
            }),
            ConstructorDescriptor::new(
                "HorizontalRadioGroup",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                        Ok(ComponentPayload::new(RadioGroupPayload {
                            id: id.clone(),
                            axis: Axis::Horizontal,
                        }))
                    }
                    _ => Err("HorizontalRadioGroup(id) expects a nonempty string id".into()),
                },
            ),
        ],
        methods: vec![
            MethodDescriptor::new(
                "selectedIndex",
                vec![ArgumentDescriptor::new("index", ArgumentSchema::Number)],
                |arguments| {
                    index_payload(
                        arguments,
                        "RadioGroup.selectedIndex(index)",
                        RadioGroupOp::Selected,
                    )
                },
            )
            .documented("Controls the selected zero-based radio index."),
            bool_method(
                "RadioGroup",
                "disabled",
                "Disables the group.",
                RadioGroupOp::Disabled,
            ),
            index_callback_method("RadioGroup", RadioGroupOp::OnChange),
        ],
        typescript: TypeScriptDescriptor::new(
            "A controlled radio set accepting only registered Radio children.",
        ),
        materializer: Arc::new(RadioGroupMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "Tab",
        constructors: vec![ConstructorDescriptor::new("Tab", vec![], |_| {
            Ok(ComponentPayload::new(TabPayload))
        })],
        methods: vec![
            MethodDescriptor::new(
                "label",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |arguments| {
                    string_payload(arguments, "Tab.label(label)", |value| {
                        ComponentPayload::new(TabOp::Label(value))
                    })
                },
            )
            .documented("Sets the visible tab label."),
            MethodDescriptor::new(
                "ariaLabel",
                vec![ArgumentDescriptor::new("label", ArgumentSchema::String)],
                |arguments| {
                    string_payload(arguments, "Tab.ariaLabel(label)", |value| {
                        ComponentPayload::new(TabOp::AriaLabel(value))
                    })
                },
            )
            .documented("Sets the accessible tab label."),
            bool_method("Tab", "disabled", "Disables the tab.", TabOp::Disabled),
            bool_method(
                "Tab",
                "selected",
                "Controls selected state.",
                TabOp::Selected,
            ),
        ],
        typescript: TypeScriptDescriptor::new("A tab accepted only as a direct TabBar child."),
        materializer: Arc::new(TabMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "TabBar",
        constructors: vec![
            id_constructor("TabBar", TabBarPayload),
            ConstructorDescriptor::new(
                "Tabs",
                vec![ArgumentDescriptor::new("id", ArgumentSchema::String)],
                |arguments| match arguments {
                    [ComponentArgument::String(id)] if !id.trim().is_empty() => {
                        Ok(ComponentPayload::new(TabBarPayload(id.clone())))
                    }
                    _ => Err("Tabs(id) expects a nonempty string id".into()),
                },
            )
            .deprecated(
                "TabBar",
                "Use TabBar; Tabs is retained as a compatibility alias.",
            ),
        ],
        methods: vec![
            MethodDescriptor::new(
                "selectedIndex",
                vec![ArgumentDescriptor::new("index", ArgumentSchema::Number)],
                |arguments| {
                    index_payload(arguments, "TabBar.selectedIndex(index)", TabBarOp::Selected)
                },
            )
            .documented("Controls the selected zero-based tab index."),
            MethodDescriptor::new(
                "variant",
                vec![ArgumentDescriptor::new(
                    "variant",
                    ArgumentSchema::Enum(&["tab", "outline", "pill", "segmented", "underline"]),
                )],
                |arguments| match arguments {
                    [ComponentArgument::Enum(value)] => {
                        let variant = match value.as_str() {
                            "tab" => TabVariant::Tab,
                            "outline" => TabVariant::Outline,
                            "pill" => TabVariant::Pill,
                            "segmented" => TabVariant::Segmented,
                            "underline" => TabVariant::Underline,
                            _ => return Err(format!("unsupported TabBar variant `{value}`")),
                        };
                        Ok(ComponentPayload::new(TabBarOp::Variant(variant)))
                    }
                    _ => Err("TabBar.variant(variant) expects a variant literal".into()),
                },
            )
            .documented("Sets one of the component's five tab variants."),
            bool_method(
                "TabBar",
                "menu",
                "Enables the overflow menu.",
                TabBarOp::Menu,
            ),
            size_method("TabBar", TabBarOp::Size),
            index_callback_method("TabBar", TabBarOp::OnChange),
        ],
        typescript: TypeScriptDescriptor::new(
            "A typed tab list accepting only Tab children; Tabs is a deprecated alias.",
        ),
        materializer: Arc::new(TabBarMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "StepperItem",
        constructors: vec![ConstructorDescriptor::new("StepperItem", vec![], |_| {
            Ok(ComponentPayload::new(StepperItemPayload))
        })],
        methods: vec![bool_method(
            "StepperItem",
            "disabled",
            "Disables this step independently of its parent.",
            StepperItemOp::Disabled,
        )],
        typescript: TypeScriptDescriptor::new(
            "A step part accepted only as a direct Stepper child.",
        ),
        materializer: Arc::new(StepperItemMaterializer),
    })?;
    registry.register(ComponentDescriptor {
        name: "Stepper",
        constructors: vec![id_constructor("Stepper", StepperPayload)],
        methods: vec![
            MethodDescriptor::new(
                "selectedIndex",
                vec![ArgumentDescriptor::new("index", ArgumentSchema::Number)],
                |arguments| {
                    index_payload(
                        arguments,
                        "Stepper.selectedIndex(index)",
                        StepperOp::Selected,
                    )
                },
            )
            .documented("Controls the current zero-based step."),
            bool_method(
                "Stepper",
                "vertical",
                "Switches between vertical and horizontal layout.",
                StepperOp::Vertical,
            ),
            bool_method(
                "Stepper",
                "textCenter",
                "Centers each step's text in horizontal layouts.",
                StepperOp::TextCenter,
            ),
            bool_method(
                "Stepper",
                "disabled",
                "Disables every step.",
                StepperOp::Disabled,
            ),
            size_method("Stepper", StepperOp::Size),
            index_callback_method("Stepper", StepperOp::OnChange),
        ],
        typescript: TypeScriptDescriptor::new(
            "A typed progress stepper accepting only StepperItem children.",
        ),
        materializer: Arc::new(StepperMaterializer),
    })?;
    Ok(())
}
