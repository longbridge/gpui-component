use super::text_slots::{self, TextSlotRule};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, FocusHandle, IntoElement, Window,
    component::text::{
        FrontmatterPlugin, MarkdownExtensions, SelectionFormat, TextView, TextViewState,
    },
};
use nsh_core::ui::{Event, JsObject, PropEnum, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;
use std::marker::PhantomData;

#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum TextFormat {
    #[default]
    Markdown,
    Html,
}
#[derive(Clone, Copy, Default, PartialEq, PropEnum)]
enum TextSelectionFormat {
    #[default]
    Plain,
    Source,
}
#[derive(Clone, PartialEq, JsObject)]
struct TextRevealBounds {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
#[derive(Event)]
enum TextViewEvent {
    LinkClick(String),
    SelectionText(String),
    Reveal(TextRevealBounds),
    Error(String),
}

struct TextConfig {
    text: String,
    format: TextFormat,
    language: Option<String>,
    selectable: bool,
    scrollable: bool,
    stream_fade: bool,
    max_lines: Option<usize>,
    intercept_links: bool,
    selection_format: TextSelectionFormat,
    frontmatter: bool,
    mdx: bool,
    blocks: Vec<TextSlotRule>,
    inline: Vec<TextSlotRule>,
}
trait RichTextProps: Props {
    fn config(&self) -> TextConfig;
}
macro_rules! text_props {
    ($name:ident, {$($extra:tt)*}, $format:expr, $language:expr, $frontmatter:expr, $mdx:expr) => {
        #[derive(Clone, PartialEq, Props)]
        struct $name {
            #[prop(default)] text: String,
            #[prop(default = true)] selectable: bool,
            #[prop(default)] scrollable: bool,
            #[prop(default)] stream_fade: bool,
            max_lines: Option<usize>,
            #[prop(default)] intercept_links: bool,
            #[prop(default)] selection_format: TextSelectionFormat,
            #[prop(default)] block_slots: Vec<TextSlotRule>,
            #[prop(default)] inline_slots: Vec<TextSlotRule>,
            $($extra)*
        }
        impl RichTextProps for $name {
            fn config(&self) -> TextConfig {
                TextConfig { text:self.text.clone(), format:($format)(self), language:($language)(self),
                    selectable:self.selectable, scrollable:self.scrollable, stream_fade:self.stream_fade,
                    max_lines:self.max_lines, intercept_links:self.intercept_links, selection_format:self.selection_format,
                    frontmatter:($frontmatter)(self), mdx:($mdx)(self),
                    blocks:self.block_slots.clone(), inline:self.inline_slots.clone() }
            }
        }
    };
}
text_props!(TextViewProps, {
    #[prop(default)] format: TextFormat,
    #[prop(default)] frontmatter: bool,
    #[prop(default)] mdx: bool,
}, |p:&TextViewProps|p.format, |_:&TextViewProps|None,
    |p:&TextViewProps|p.frontmatter, |p:&TextViewProps|p.mdx);
text_props!(MarkdownProps, {
    #[prop(default)] format: TextFormat,
    #[prop(default)] frontmatter: bool,
    #[prop(default)] mdx: bool,
}, |p:&MarkdownProps|p.format, |_:&MarkdownProps|None,
    |p:&MarkdownProps|p.frontmatter, |p:&MarkdownProps|p.mdx);
text_props!(
    HtmlProps,
    {},
    |_: &HtmlProps| TextFormat::Html,
    |_: &HtmlProps| None,
    |_: &HtmlProps| false,
    |_: &HtmlProps| false
);
text_props!(CodeBlockProps, { language: Option<String>, },
    |_:&CodeBlockProps|TextFormat::Markdown, |p:&CodeBlockProps|Some(p.language.clone().unwrap_or_default()),
    |_:&CodeBlockProps|false, |_:&CodeBlockProps|false);

pub(super) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<RichText<TextViewProps>>(
        "gpui-component.TextView",
        "TextView",
        RichText::new,
    );
    registry.register::<RichText<MarkdownProps>>(
        "gpui-component.Markdown",
        "Markdown",
        RichText::new,
    );
    registry.register::<RichText<HtmlProps>>("gpui-component.Html", "Html", RichText::new);
    registry.register::<RichText<CodeBlockProps>>(
        "gpui-component.CodeBlock",
        "CodeBlock",
        RichText::new,
    );
}
struct RichText<P: RichTextProps> {
    context: TypedComponentContext<TextViewEvent>,
    state: Option<Entity<TextViewState>>,
    source: String,
    focus: Option<FocusHandle>,
    format: TextFormat,
    props: PhantomData<P>,
}
impl<P: RichTextProps> RichText<P> {
    fn new(context: TypedComponentContext<TextViewEvent>) -> Self {
        Self {
            context,
            state: None,
            source: String::new(),
            focus: None,
            format: TextFormat::Markdown,
            props: PhantomData,
        }
    }
}
impl<P: RichTextProps> TypedNativeComponent for RichText<P> {
    type Props = P;
    type Event = TextViewEvent;
    fn render(&mut self, props: &P, _: &mut Window, cx: &mut App) -> Option<AnyElement> {
        let config = props.config();
        let mut source = config.text;
        if let Some(language) = config.language {
            let fence = "`".repeat(
                source
                    .split(|c| c != '`')
                    .map(str::len)
                    .max()
                    .unwrap_or(0)
                    .max(2)
                    + 1,
            );
            let language = language.replace(['\r', '\n', '`'], "");
            source = format!("{fence}{language}\n{source}\n{fence}");
        }
        match (self.state.as_ref(), self.format == config.format) {
            (Some(state), true) => {
                if self.source != source {
                    state.update(cx, |state, cx| state.set_text(&source, cx));
                }
            }
            _ => {
                self.state = Some(cx.new(|cx| match config.format {
                    TextFormat::Html => TextViewState::html(&source, cx),
                    TextFormat::Markdown => TextViewState::markdown(&source, cx),
                }));
                self.format = config.format;
            }
        }
        self.source = source;
        self.focus = self
            .state
            .as_ref()
            .map(|state| state.read(cx).focus_handle().clone());
        let mut extensions = MarkdownExtensions::default();
        if config.mdx {
            extensions = extensions.mdx();
        }
        if config.frontmatter {
            extensions = extensions.frontmatter().plugin(FrontmatterPlugin::new());
        }
        let extensions = text_slots::extend(
            extensions,
            &config.blocks,
            &config.inline,
            config.frontmatter,
            config.mdx,
            &self.context,
        );
        let reveal = self.context.clone();
        let context = self.context.clone();
        let view = TextView::new(self.state.as_ref().unwrap())
            .markdown_extensions(extensions)
            .on_reveal(move |bounds, _, _| {
                reveal.emit(TextViewEvent::Reveal(TextRevealBounds {
                    x: bounds.origin.x.into(),
                    y: bounds.origin.y.into(),
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                }))
            })
            .selectable(config.selectable)
            .scrollable(config.scrollable)
            .stream_fade(config.stream_fade)
            .selection_format(match config.selection_format {
                TextSelectionFormat::Plain => SelectionFormat::Plain,
                TextSelectionFormat::Source => SelectionFormat::Source,
            });
        let view = if config.intercept_links {
            view.on_link_click(move |url, _, _, _| {
                context.emit(TextViewEvent::LinkClick(url.to_string()))
            })
        } else {
            view
        };
        Some(
            if let Some(lines) = config.max_lines {
                view.max_lines(lines)
            } else {
                view
            }
            .into_any_element(),
        )
    }
    fn focus_handle(&self) -> Option<FocusHandle> {
        self.focus.clone()
    }
    fn command(&mut self, _: &P, name: &str, args: &Value, window: &mut Window, cx: &mut App) {
        let Some(state) = &self.state else { return };
        if name == "focus" {
            let focus = state.read(cx).focus_handle().clone();
            window.focus(&focus, cx);
            return;
        }
        if name == "getSelectedText" {
            self.context
                .emit(TextViewEvent::SelectionText(state.read(cx).selected_text()));
            return;
        }
        state.update(cx, |state, cx| match name {
            "selectAll" => state.select_all(cx),
            "clearSelection" => state.clear_selection(cx),
            "append" => state.push_str(args.as_str().unwrap_or_default(), cx),
            "revealRange" => {
                if let Err(error) = state.reveal_range(
                    args["start"].as_u64().unwrap_or(0) as usize
                        ..args["end"].as_u64().unwrap_or(0) as usize,
                    cx,
                ) {
                    self.context.emit(TextViewEvent::Error(error.to_string()));
                }
            }
            _ => {}
        });
    }
}
