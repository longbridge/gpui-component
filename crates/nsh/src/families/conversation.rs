//! Chat surfaces and retained message scrolling, composed from real NSH slots.
use crate::bridge::{children, slot};
use crate::registration::native_leaf;
use gpui_kit::component::{
    TitleBar, WindowBorder,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
        AttachmentMedia, AttachmentStatus, AttachmentTitle,
    },
    bubble::{
        Bubble, BubbleContent, BubbleGroup, BubbleReactionSide, BubbleReactions, BubbleVariant,
    },
    message::{
        Message, MessageAlignment, MessageAvatar, MessageContent, MessageFooter, MessageGroup,
        MessageHeader,
    },
    message_scroller::{MessageScroller, MessageScrollerState},
};
use gpui_kit::{
    AnyElement, App, AppContext, Entity, IntoElement, ParentElement, Window, div,
    prelude::FluentBuilder,
};
use nsh_core::ui::{Children, Event, PropEnum, PropError, Props};
use nsh_platform_gpui::{TypedComponentContext, TypedNativeComponent};
use serde_json::Value;

#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum ConversationAlignment {
    Start,
    End,
}
impl ConversationAlignment {
    fn native(self) -> MessageAlignment {
        match self {
            Self::Start => MessageAlignment::Start,
            Self::End => MessageAlignment::End,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum NativeBubbleVariant {
    Filled,
    Secondary,
    Muted,
    Tinted,
    Outline,
    Ghost,
    Destructive,
}
impl NativeBubbleVariant {
    fn native(self) -> BubbleVariant {
        match self {
            Self::Filled => BubbleVariant::Filled,
            Self::Secondary => BubbleVariant::Secondary,
            Self::Muted => BubbleVariant::Muted,
            Self::Tinted => BubbleVariant::Tinted,
            Self::Outline => BubbleVariant::Outline,
            Self::Ghost => BubbleVariant::Ghost,
            Self::Destructive => BubbleVariant::Destructive,
        }
    }
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum NativeReactionSide {
    Top,
    Bottom,
}
#[derive(Clone, Copy, PartialEq, PropEnum)]
pub(crate) enum AttachmentUploadStatus {
    Pending,
    Uploading,
    Processing,
    Failed,
    Complete,
}
impl AttachmentUploadStatus {
    fn native(self) -> AttachmentStatus {
        match self {
            Self::Pending => AttachmentStatus::Pending,
            Self::Uploading => AttachmentStatus::Uploading,
            Self::Processing => AttachmentStatus::Processing,
            Self::Failed => AttachmentStatus::Failed,
            Self::Complete => AttachmentStatus::Complete,
        }
    }
}
native_leaf!(NativeMessageGroup(MessageGroupProps, context, cx, p) =>
    MessageGroup::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeBubbleGroup(BubbleGroupProps, context, cx, p) =>
    BubbleGroup::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeAttachmentGroup(AttachmentGroupProps, context, cx, p) =>
    AttachmentGroup::new("attachments").children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeWindowBorder(WindowBorderProps, context, cx, p) =>
    WindowBorder::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeMessageAvatar(MessageAvatarProps, context, cx, p) =>
    MessageAvatar::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeMessageContent(MessageContentProps, context, cx, p) =>
    MessageContent::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeBubbleContent(BubbleContentProps, context, cx, p) =>
    BubbleContent::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeAttachmentActions(AttachmentActionsProps, context, cx, p) =>
    AttachmentActions::new().children(children(context));
{
    children: Children => skip;
});
native_leaf!(NativeAttachmentMedia(AttachmentMediaProps, context, cx, p) =>
    AttachmentMedia::new()
        .children(children(context))
        .when_some(slot(context, "overlay"), |this, overlay| {
            this.overlay(overlay)
        });
{
    children: Children => skip;
});
native_leaf!(NativeMessageHeader(MessageHeaderProps, context, cx, p) =>
    MessageHeader::new()
        .content_inset(p.content_inset.unwrap_or(true))
        .children(children(context));
{
    content_inset: Option<bool> => skip;
    children: Children => skip;
});
native_leaf!(NativeMessageFooter(MessageFooterProps, context, cx, p) =>
    MessageFooter::new()
        .content_inset(p.content_inset.unwrap_or(true))
        .children(children(context));
{
    content_inset: Option<bool> => skip;
    children: Children => skip;
});
native_leaf!(NativeAttachmentTitle(AttachmentTitleProps, context, cx, p) =>
    AttachmentTitle::new(p.text.clone().unwrap_or_default());
{
    text: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeAttachmentDescription(AttachmentDescriptionProps, context, cx, p) =>
    AttachmentDescription::new(p.text.clone().unwrap_or_default());
{
    text: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeAttachmentContent(AttachmentContentProps, context, cx, p) =>
    AttachmentContent::new()
        .title(AttachmentTitle::new(p.title.clone().unwrap_or_default()))
        .description(AttachmentDescription::new(
            p.description.clone().unwrap_or_default(),
        ))
        .children(children(context));
{
    title: Option<String> => skip;
    description: Option<String> => skip;
    children: Children => skip;
});
native_leaf!(NativeBubbleReactions(BubbleReactionsProps, context, cx, p) =>
    BubbleReactions::new()
        .alignment(p.alignment.unwrap_or(ConversationAlignment::Start).native())
        .side(match p.side {
            Some(NativeReactionSide::Top) => BubbleReactionSide::Top,
            _ => BubbleReactionSide::Bottom,
        })
        .children(children(context));
{
    alignment: Option<ConversationAlignment> => skip;
    side: Option<NativeReactionSide> => skip;
    children: Children => skip;
});
native_leaf!(NativeMessage(MessageProps, context, cx, p) =>
    Message::new()
        .id("message")
        .alignment(p.alignment.unwrap_or(ConversationAlignment::Start).native())
        .content(MessageContent::new().children(children(context)))
        .when_some(slot(context, "avatar"), |this, child| this.avatar(child))
        .when_some(slot(context, "header"), |this, child| {
            this.header(MessageHeader::new().child(child))
        })
        .when_some(slot(context, "footer"), |this, child| {
            this.footer(MessageFooter::new().child(child))
        });
{
    alignment: Option<ConversationAlignment> => skip;
    children: Children => skip;
});
native_leaf!(NativeBubble(BubbleProps, context, cx, p) =>
    Bubble::new()
        .alignment(p.alignment.unwrap_or(ConversationAlignment::Start).native())
        .with_variant(p.variant.unwrap_or(NativeBubbleVariant::Filled).native())
        .children(children(context))
        .when_some(slot(context, "reactions"), |this, child| {
            this.reactions(BubbleReactions::new().child(child))
        });
{
    alignment: Option<ConversationAlignment> => skip;
    variant: Option<NativeBubbleVariant> => skip;
    children: Children => skip;
});

#[derive(Clone, PartialEq, Props)]
pub(crate) struct AttachmentProps {
    title: String,
    description: Option<String>,
    status: Option<AttachmentUploadStatus>,
    progress: Option<f64>,
    #[prop(default)]
    removable: bool,
    #[prop(default)]
    retryable: bool,
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum AttachmentEvent {
    Press,
    Remove,
    Retry,
}
struct NativeAttachment {
    context: TypedComponentContext<AttachmentEvent>,
}
impl TypedNativeComponent for NativeAttachment {
    type Props = AttachmentProps;
    type Event = AttachmentEvent;
    fn render(&mut self, p: &AttachmentProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let context = self.context.clone();
        Some(
            Attachment::new()
                .id("attachment")
                .status(
                    p.status
                        .unwrap_or(AttachmentUploadStatus::Complete)
                        .native(),
                )
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(p.title.clone()))
                        .description(AttachmentDescription::new(
                            p.description.clone().unwrap_or_default(),
                        ))
                        .children(children(&context)),
                )
                .when_some(slot(&context, "media"), |this, child| {
                    this.media(AttachmentMedia::new().child(child))
                })
                .when_some(slot(&context, "actions"), |this, child| {
                    this.actions(AttachmentActions::new().child(child))
                })
                .when_some(p.progress, |this, value| {
                    this.progress(value.clamp(0., 100.) as f32)
                })
                .on_click({
                    let context = context.clone();
                    move |_, _, _| context.emit(AttachmentEvent::Press)
                })
                .when(p.removable, |this| {
                    let context = context.clone();
                    this.on_remove(move |_, _, _| context.emit(AttachmentEvent::Remove))
                })
                .when(p.retryable, |this| {
                    this.on_retry(move |_, _, _| context.emit(AttachmentEvent::Retry))
                })
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct TitleBarProps {
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum TitleBarEvent {
    Close,
}
struct NativeTitleBar {
    context: TypedComponentContext<TitleBarEvent>,
}
impl TypedNativeComponent for NativeTitleBar {
    type Props = TitleBarProps;
    type Event = TitleBarEvent;
    fn render(&mut self, _: &TitleBarProps, _: &mut Window, _: &mut App) -> Option<AnyElement> {
        let context = self.context.clone();
        Some(
            TitleBar::new()
                .children(children(&context))
                .on_close_window(move |_, _, _| context.emit(TitleBarEvent::Close))
                .into_any_element(),
        )
    }
}
#[derive(Clone, PartialEq, Props)]
pub(crate) struct MessageScrollerProps {
    items: Vec<String>,
    scrollbar: Option<bool>,
    jump_button: Option<bool>,
    children: Children,
}
#[derive(Clone, Event)]
pub(crate) enum ScrollerEvent {
    // The shared typed host emits validation failures.
    #[allow(dead_code)]
    Error(String),
}
struct NativeMessageScroller {
    context: TypedComponentContext<ScrollerEvent>,
    keys: Vec<String>,
    scroller: Option<Entity<MessageScrollerState>>,
}
impl TypedNativeComponent for NativeMessageScroller {
    type Props = MessageScrollerProps;
    type Event = ScrollerEvent;
    fn validate_props(&self, p: &MessageScrollerProps) -> Result<(), PropError> {
        let mut seen = std::collections::HashSet::new();
        if p.items.iter().any(|id| id.is_empty() || !seen.insert(id)) {
            return Err(PropError(
                "MessageScroller item ids must be unique nonempty strings".into(),
            ));
        }
        Ok(())
    }
    fn render(
        &mut self,
        p: &MessageScrollerProps,
        _: &mut Window,
        cx: &mut App,
    ) -> Option<AnyElement> {
        let keys = p.items.clone();
        let context = self.context.clone();
        let state = self
            .scroller
            .get_or_insert_with(|| cx.new(|cx| MessageScrollerState::new(0, cx)))
            .clone();
        if self.keys != keys {
            state.update(cx, |state, cx| {
                if keys.starts_with(&self.keys) {
                    state.append(keys.len() - self.keys.len(), cx);
                } else if keys.ends_with(&self.keys) {
                    state.prepend(keys.len() - self.keys.len(), cx);
                } else {
                    state.reset(keys.len(), cx);
                }
            });
            self.keys = keys.clone();
        }
        Some(
            MessageScroller::new("messages", state, move |index, _, _| {
                keys.get(index)
                    .and_then(|id| slot(&context, id))
                    .unwrap_or_else(|| div().into_any_element())
            })
            .scrollbar(p.scrollbar.unwrap_or(true))
            .jump_button(p.jump_button.unwrap_or(true))
            .into_any_element(),
        )
    }
    fn command(
        &mut self,
        _: &MessageScrollerProps,
        name: &str,
        _: &Value,
        _: &mut Window,
        cx: &mut App,
    ) {
        if name == "scrollToEnd" {
            if let Some(state) = &self.scroller {
                state.update(cx, |state, cx| state.scroll_to_end(cx));
            }
        }
    }
}
pub(crate) fn register_typed(registry: &mut impl crate::registration::BindingRegistry) {
    registry.register::<NativeMessageGroup>(
        "gpui-component.MessageGroup",
        "MessageGroup",
        NativeMessageGroup::new,
    );
    registry.register::<NativeBubbleGroup>(
        "gpui-component.BubbleGroup",
        "BubbleGroup",
        NativeBubbleGroup::new,
    );
    registry.register::<NativeAttachmentGroup>(
        "gpui-component.AttachmentGroup",
        "AttachmentGroup",
        NativeAttachmentGroup::new,
    );
    registry.register::<NativeWindowBorder>(
        "gpui-component.WindowBorder",
        "WindowBorder",
        NativeWindowBorder::new,
    );
    registry.register::<NativeMessageAvatar>(
        "gpui-component.MessageAvatar",
        "MessageAvatar",
        NativeMessageAvatar::new,
    );
    registry.register::<NativeMessageContent>(
        "gpui-component.MessageContent",
        "MessageContent",
        NativeMessageContent::new,
    );
    registry.register::<NativeBubbleContent>(
        "gpui-component.BubbleContent",
        "BubbleContent",
        NativeBubbleContent::new,
    );
    registry.register::<NativeAttachmentActions>(
        "gpui-component.AttachmentActions",
        "AttachmentActions",
        NativeAttachmentActions::new,
    );
    registry.register::<NativeAttachmentMedia>(
        "gpui-component.AttachmentMedia",
        "AttachmentMedia",
        NativeAttachmentMedia::new,
    );
    registry.register::<NativeMessageHeader>(
        "gpui-component.MessageHeader",
        "MessageHeader",
        NativeMessageHeader::new,
    );
    registry.register::<NativeMessageFooter>(
        "gpui-component.MessageFooter",
        "MessageFooter",
        NativeMessageFooter::new,
    );
    registry.register::<NativeAttachmentTitle>(
        "gpui-component.AttachmentTitle",
        "AttachmentTitle",
        NativeAttachmentTitle::new,
    );
    registry.register::<NativeAttachmentDescription>(
        "gpui-component.AttachmentDescription",
        "AttachmentDescription",
        NativeAttachmentDescription::new,
    );
    registry.register::<NativeAttachmentContent>(
        "gpui-component.AttachmentContent",
        "AttachmentContent",
        NativeAttachmentContent::new,
    );
    registry.register::<NativeBubbleReactions>(
        "gpui-component.BubbleReactions",
        "BubbleReactions",
        NativeBubbleReactions::new,
    );
    registry.register::<NativeMessage>("gpui-component.Message", "Message", NativeMessage::new);
    registry.register::<NativeBubble>("gpui-component.Bubble", "Bubble", NativeBubble::new);
    registry.register::<NativeAttachment>("gpui-component.Attachment", "Attachment", |context| {
        NativeAttachment { context }
    });
    registry.register::<NativeTitleBar>("gpui-component.TitleBar", "TitleBar", |context| {
        NativeTitleBar { context }
    });
    registry.register::<NativeMessageScroller>(
        "gpui-component.MessageScroller",
        "MessageScroller",
        |context| NativeMessageScroller {
            context,
            keys: vec![],
            scroller: None,
        },
    );
}
