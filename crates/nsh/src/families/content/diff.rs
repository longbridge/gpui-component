use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, Focusable, IntoElement, Subscription, Window,
    component::diff::{
        Diff, DiffAnnotation, DiffChangeIndicator, DiffEvent as NativeDiffEvent, DiffFile,
        DiffHoverHighlight, DiffHunkSeparator, DiffInlineUnit, DiffLinePosition, DiffMode,
        DiffSide, DiffState,
    },
    div,
    prelude::FluentBuilder,
};
use nsh_core::ui::{Either, Event, JsObject, PropEnum, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffPresentation {
    Split,
    #[default]
    Unified,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffInlineGranularity {
    None,
    #[default]
    Word,
    Character,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffHoverStyle {
    #[default]
    None,
    Line,
    LineNumber,
    Both,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffHunkStyle {
    #[default]
    Metadata,
    LineInfo,
    Simple,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffIndicatorStyle {
    #[default]
    Signs,
    Bars,
    None,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum DiffDocumentSide {
    Original,
    #[default]
    Modified,
}
impl From<DiffSide> for DiffDocumentSide {
    fn from(side: DiffSide) -> Self {
        match side {
            DiffSide::Original => Self::Original,
            DiffSide::Modified => Self::Modified,
        }
    }
}
impl From<DiffDocumentSide> for DiffSide {
    fn from(side: DiffDocumentSide) -> Self {
        match side {
            DiffDocumentSide::Original => Self::Original,
            DiffDocumentSide::Modified => Self::Modified,
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffLineRange {
    path: String,
    side: DiffDocumentSide,
    start: usize,
    end_side: DiffDocumentSide,
    end: usize,
}
impl From<&gpui_kit::component::diff::DiffLineRange> for DiffLineRange {
    fn from(range: &gpui_kit::component::diff::DiffLineRange) -> Self {
        Self {
            path: range.path().to_string(),
            side: range.side().into(),
            start: range.start(),
            end_side: range.end_side().into(),
            end: range.end(),
        }
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffLineTarget {
    path: String,
    line: usize,
    side: DiffDocumentSide,
}
impl From<&DiffLinePosition> for DiffLineTarget {
    fn from(position: &DiffLinePosition) -> Self {
        Self {
            path: position.path().to_string(),
            line: position.line(),
            side: position.side().into(),
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DiffSelectionBoundary {
    SelectionStarted,
    SelectionEnded,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DiffSelectionChangeTag {
    SelectionChanged,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DiffFileChangeTag {
    FileExpanded,
    FileCollapsed,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
enum DiffConflictChangeTag {
    ConflictResolved,
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffBoundaryChange {
    #[prop(rename = "type")]
    change_type: DiffSelectionBoundary,
    range: DiffLineRange,
}
crate::registration::named_value! {struct DiffNullableRange(Option<DiffLineRange>);}
#[derive(Clone, PartialEq, JsObject)]
struct DiffSelectionChange {
    #[prop(rename = "type")]
    change_type: DiffSelectionChangeTag,
    range: DiffNullableRange,
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffFileChange {
    #[prop(rename = "type")]
    change_type: DiffFileChangeTag,
    path: String,
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffConflictChange {
    #[prop(rename = "type")]
    change_type: DiffConflictChangeTag,
    path: String,
    index: usize,
}
crate::registration::named_value! {struct DiffChange(Either<DiffBoundaryChange,Either<DiffSelectionChange,Either<DiffFileChange,DiffConflictChange>>>);}
impl DiffChange {
    fn native(event: &NativeDiffEvent) -> Option<Self> {
        Some(Self::new(match event {
            NativeDiffEvent::SelectionStarted(range) => Either::Left(DiffBoundaryChange {
                change_type: DiffSelectionBoundary::SelectionStarted,
                range: range.into(),
            }),
            NativeDiffEvent::SelectionEnded(range) => Either::Left(DiffBoundaryChange {
                change_type: DiffSelectionBoundary::SelectionEnded,
                range: range.into(),
            }),
            NativeDiffEvent::SelectionChanged(range) => {
                Either::Right(Either::Left(DiffSelectionChange {
                    change_type: DiffSelectionChangeTag::SelectionChanged,
                    range: DiffNullableRange::new(range.as_ref().map(Into::into)),
                }))
            }
            NativeDiffEvent::FileExpanded(path) => {
                Either::Right(Either::Right(Either::Left(DiffFileChange {
                    change_type: DiffFileChangeTag::FileExpanded,
                    path: path.to_string(),
                })))
            }
            NativeDiffEvent::FileCollapsed(path) => {
                Either::Right(Either::Right(Either::Left(DiffFileChange {
                    change_type: DiffFileChangeTag::FileCollapsed,
                    path: path.to_string(),
                })))
            }
            NativeDiffEvent::ConflictResolved(path, index) => {
                Either::Right(Either::Right(Either::Right(DiffConflictChange {
                    change_type: DiffConflictChangeTag::ConflictResolved,
                    path: path.to_string(),
                    index: *index,
                })))
            }
            _ => return None,
        }))
    }
}
#[derive(Clone, PartialEq, JsObject)]
struct DiffAnnotationData {
    id: String,
    path: String,
    side: Option<DiffDocumentSide>,
    line: Option<usize>,
}
#[derive(Clone, PartialEq, Props)]
struct DiffProps {
    #[prop(default)]
    patch: String,
    #[prop(default)]
    mode: DiffPresentation,
    context_lines: Option<usize>,
    #[prop(default = 20)]
    expansion_lines: usize,
    #[prop(default = 2)]
    min_collapsed_lines: usize,
    #[prop(default)]
    inline_unit: DiffInlineGranularity,
    #[prop(default = 1000)]
    inline_max_line_length: usize,
    #[prop(default = 1000)]
    syntax_max_line_length: usize,
    #[prop(default = true)]
    line_number: bool,
    #[prop(default = true)]
    syntax_highlight: bool,
    #[prop(default = true)]
    header_visible: bool,
    #[prop(default)]
    soft_wrap: bool,
    #[prop(default)]
    annotations: Vec<DiffAnnotationData>,
    #[prop(default)]
    hover_highlight: DiffHoverStyle,
    #[prop(default = true)]
    change_background: bool,
    #[prop(default)]
    annotatable: bool,
    #[prop(default)]
    hunk_separator: DiffHunkStyle,
    #[prop(default)]
    change_indicator: DiffIndicatorStyle,
}
#[derive(Event)]
enum DiffBindingEvent {
    Change(DiffChange),
    LineClick(DiffLineTarget),
    LineHover(Option<DiffLineTarget>),
    AddAnnotation(DiffLineRange),
    SelectionText(String),
    Error(String),
}

pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeDiff>("gpui-component.Diff", "Diff", NativeDiff::new);
}
struct NativeDiff {
    context: TypedComponentContext<DiffBindingEvent>,
    state: Option<Entity<DiffState>>,
    focus: Option<FocusHandle>,
    patch: String,
    subscription: Option<Subscription>,
}
impl NativeDiff {
    fn new(context: TypedComponentContext<DiffBindingEvent>) -> Self {
        Self {
            context,
            state: None,
            focus: None,
            patch: String::new(),
            subscription: None,
        }
    }
}
impl TypedNativeComponent for NativeDiff {
    type Props = DiffProps;
    type Event = DiffBindingEvent;
    fn validate_props(&self, props: &DiffProps) -> Result<(), PropError> {
        if self.patch != props.patch {
            DiffFile::parse(&props.patch).map_err(|error| {
                PropError(format!(
                    "Invalid patch at line {}: {}",
                    error.line(),
                    error.message()
                ))
            })?;
        }
        Ok(())
    }
    fn render(
        &mut self,
        props: &DiffProps,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let state = self
            .state
            .get_or_insert_with(|| cx.new(|cx| DiffState::new([], cx)))
            .clone();
        self.focus = Some(state.read(cx).focus_handle(cx));
        if self.subscription.is_none() {
            let context = self.context.clone();
            self.subscription =
                Some(
                    window.subscribe(&state, cx, move |_, event: &NativeDiffEvent, _, _| {
                        if let Some(change) = DiffChange::native(event) {
                            context.emit(DiffBindingEvent::Change(change));
                            context.redraw();
                        }
                    }),
                );
        }
        if self.patch != props.patch {
            match DiffFile::parse(&props.patch) {
                Ok(files) => state.update(cx, |state, cx| state.set_files(files, cx)),
                Err(error) => self
                    .context
                    .emit(DiffBindingEvent::Error(error.message().to_string())),
            };
            self.patch = props.patch.clone();
        }
        state.update(cx, |state, cx| {
            let mode = match props.mode {
                DiffPresentation::Split => DiffMode::Split,
                DiffPresentation::Unified => DiffMode::Unified,
            };
            if state.mode() != mode {
                state.set_mode(mode, cx);
            }
            state.set_expansion_lines(props.expansion_lines, cx);
            state.set_min_collapsed_lines(props.min_collapsed_lines, cx);
            state.set_inline_max_line_length(props.inline_max_line_length, cx);
            state.set_syntax_max_line_length(props.syntax_max_line_length, cx);
            state.set_inline_unit(
                match props.inline_unit {
                    DiffInlineGranularity::None => None,
                    DiffInlineGranularity::Word => Some(DiffInlineUnit::Word),
                    DiffInlineGranularity::Character => Some(DiffInlineUnit::Character),
                },
                cx,
            );
            if state.context_lines() != props.context_lines {
                state.set_context_lines(props.context_lines, cx);
            }
        });
        let annotations = props
            .annotations
            .iter()
            .map(|annotation| {
                if let Some(line) = annotation.line {
                    DiffAnnotation::line(
                        annotation.id.clone(),
                        DiffLinePosition::new(
                            annotation.path.clone(),
                            annotation.side.unwrap_or_default().into(),
                            line,
                        ),
                    )
                } else {
                    DiffAnnotation::file(annotation.id.clone(), annotation.path.clone())
                }
            })
            .collect::<Vec<_>>();
        let annotation_ids = props
            .annotations
            .iter()
            .map(|a| (gpui_kit::ElementId::Name(a.id.clone().into()), a.id.clone()))
            .collect::<HashMap<_, _>>();
        let context = self.context.clone();
        let prefix = context.clone();
        let title_suffix = context.clone();
        let suffix = context.clone();
        let hover = context.clone();
        let annotate = context.clone();
        let annotation_context = context.clone();
        Some(
            Diff::new(&state)
                .annotations(annotations)
                .render_annotation(move |annotation, _, _| {
                    annotation_ids
                        .get(annotation.id())
                        .and_then(|id| {
                            crate::bridge::slot(&annotation_context, &format!("annotation:{id}"))
                        })
                        .unwrap_or_else(|| div().into_any_element())
                })
                .render_header_prefix(move |file, _, _| {
                    crate::bridge::slot(&prefix, &format!("headerPrefix:{}", file.path()))
                        .unwrap_or_else(|| div().into_any_element())
                })
                .render_header_title_suffix(move |file, _, _| {
                    crate::bridge::slot(
                        &title_suffix,
                        &format!("headerTitleSuffix:{}", file.path()),
                    )
                    .unwrap_or_else(|| div().into_any_element())
                })
                .render_header_suffix(move |file, _, _| {
                    crate::bridge::slot(&suffix, &format!("headerSuffix:{}", file.path()))
                        .unwrap_or_else(|| div().into_any_element())
                })
                .on_line_hover(move |position, _, _| {
                    hover.emit(DiffBindingEvent::LineHover(position.map(Into::into)))
                })
                .when(props.annotatable, |diff| {
                    diff.on_add_annotation(move |range, _, _| {
                        annotate.emit(DiffBindingEvent::AddAnnotation(range.into()))
                    })
                })
                .hover_highlight(match props.hover_highlight {
                    DiffHoverStyle::None => DiffHoverHighlight::None,
                    DiffHoverStyle::Line => DiffHoverHighlight::Line,
                    DiffHoverStyle::LineNumber => DiffHoverHighlight::LineNumber,
                    DiffHoverStyle::Both => DiffHoverHighlight::Both,
                })
                .change_background(props.change_background)
                .hunk_separator(match props.hunk_separator {
                    DiffHunkStyle::Metadata => DiffHunkSeparator::Metadata,
                    DiffHunkStyle::LineInfo => DiffHunkSeparator::LineInfo,
                    DiffHunkStyle::Simple => DiffHunkSeparator::Simple,
                })
                .change_indicator(match props.change_indicator {
                    DiffIndicatorStyle::Signs => DiffChangeIndicator::Signs,
                    DiffIndicatorStyle::Bars => DiffChangeIndicator::Bars,
                    DiffIndicatorStyle::None => DiffChangeIndicator::None,
                })
                .line_number(props.line_number)
                .syntax_highlight(props.syntax_highlight)
                .header_visible(props.header_visible)
                .soft_wrap(props.soft_wrap)
                .on_line_click(move |position, _, _, _| {
                    context.emit(DiffBindingEvent::LineClick(position.into()))
                })
                .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(
        &mut self,
        _: &DiffProps,
        name: &str,
        args: &Value,
        window: &mut Window,
        cx: &mut App,
    ) {
        let Some(state) = &self.state else { return };
        if name == "focus" {
            if let Some(focus) = &self.focus {
                window.focus(focus, cx);
            }
            return;
        }
        if name == "getSelectedText" {
            self.context.emit(DiffBindingEvent::SelectionText(
                state.read(cx).selected_text(cx),
            ));
            return;
        }
        state.update(cx, |state, cx| match name {
            "nextChange" => state.next_change(cx),
            "previousChange" => state.previous_change(cx),
            "expandUnchanged" => state.expand_unchanged(cx),
            "collapseUnchanged" => state.collapse_unchanged(cx),
            "scrollToFile" => state.scroll_to_file(args.as_str().unwrap_or_default(), cx),
            "scrollToLine" => state.scroll_to_line(
                DiffLinePosition::new(
                    args["path"].as_str().unwrap_or_default().to_owned(),
                    if args["side"].as_str() == Some("original") {
                        DiffSide::Original
                    } else {
                        DiffSide::Modified
                    },
                    args["line"].as_u64().unwrap_or(1) as usize,
                ),
                cx,
            ),
            "setFileCollapsed" => state.set_file_collapsed(
                args["path"].as_str().unwrap_or_default(),
                args["collapsed"].as_bool().unwrap_or(false),
                cx,
            ),
            "resolveConflict" => state.resolve_conflict(
                args["path"].as_str().unwrap_or_default(),
                args["index"].as_u64().unwrap_or(0) as usize,
                match args["resolution"].as_str() {
                    Some("current") => {
                        Some(gpui_kit::component::diff::DiffConflictResolution::Current)
                    }
                    Some("incoming") => {
                        Some(gpui_kit::component::diff::DiffConflictResolution::Incoming)
                    }
                    Some("both") => Some(gpui_kit::component::diff::DiffConflictResolution::Both),
                    _ => None,
                },
                cx,
            ),
            _ => {}
        });
    }
}
