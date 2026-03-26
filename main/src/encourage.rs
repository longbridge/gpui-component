// ============================================================================
// 支持作者页签面板
// ============================================================================

use gpui::{
    AnyElement, App, ClickEvent, FontWeight, Image, ImageFormat, IntoElement, ParentElement,
    SharedString, Styled, StyledImage, div, img, px,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::clipboard::Clipboard;
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, h_flex, v_flex};
use rust_i18n::t;
use std::sync::Arc;

// TODO 修改赞赏码和地址

const WECHAT_QR_ENV: &str = "ONETCLI_WECHAT_QR_URL";
const ALIPAY_QR_ENV: &str = "ONETCLI_ALIPAY_QR_URL";
const PAYPAL_QR_ENV: &str = "ONETCLI_PAYPAL_QR_URL";

const WECHAT_QR_BUILD: Option<&str> = option_env!("ONETCLI_WECHAT_QR_URL");
const ALIPAY_QR_BUILD: Option<&str> = option_env!("ONETCLI_ALIPAY_QR_URL");
const PAYPAL_QR_BUILD: Option<&str> = option_env!("ONETCLI_PAYPAL_QR_URL");

const WECHAT_QR_OFFLINE: &[u8] = include_bytes!("../assets/encourage/wechat.png");
const ALIPAY_QR_OFFLINE: &[u8] = include_bytes!("../assets/encourage/alipay.png");
const PAYPAL_QR_OFFLINE: &[u8] = include_bytes!("../assets/encourage/paypal.png");

const GITHUB_URL: &str = "https://github.com/feigeCode/onetcli";

pub(crate) fn render_encourage_section(cx: &App) -> AnyElement {
    EncourageContent::load().render_settings_section(cx)
}

struct EncourageContent {
    urls: EncourageQrUrls,
    offline_images: EncourageQrImages,
}

impl EncourageContent {
    fn load() -> Self {
        Self {
            urls: EncourageQrUrls::load(),
            offline_images: EncourageQrImages::load(),
        }
    }

    fn section_card(
        &self,
        icon: IconName,
        title: SharedString,
        body: AnyElement,
        cx: &App,
    ) -> AnyElement {
        div()
            .w_full()
            .rounded_xl()
            .border_1()
            .border_color(cx.theme().border)
            .bg(gpui::hsla(0.0, 0.0, 0.5, 0.05))
            .p_5()
            .child(
                v_flex()
                    .w_full()
                    .gap_4()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(36.0))
                                    .h(px(36.0))
                                    .rounded_xl()
                                    .bg(gpui::hsla(0.0, 0.0, 0.5, 0.08))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        Icon::new(icon)
                                            .with_size(px(18.0))
                                            .text_color(cx.theme().link),
                                    ),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(title),
                            ),
                    )
                    .child(body),
            )
            .into_any_element()
    }

    fn render_intro(&self, cx: &App) -> AnyElement {
        let body = v_flex()
            .w_full()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .child(t!("Encourage.intro_line1").to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Encourage.intro_line2").to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Encourage.intro_line3").to_string()),
            )
            .into_any_element();

        self.section_card(IconName::Star, t!("Encourage.title").into(), body, cx)
    }

    fn render_alt_support_section(&self, cx: &App) -> AnyElement {
        let body = v_flex()
            .w_full()
            .gap_3()
            .children(
                [
                    t!("Encourage.alt_support_item1"),
                    t!("Encourage.alt_support_item2"),
                    t!("Encourage.alt_support_item3"),
                ]
                .into_iter()
                .map(|item| {
                    h_flex()
                        .w_full()
                        .items_center()
                        .gap_2()
                        .child(
                            Icon::new(IconName::CircleCheck)
                                .with_size(px(16.0))
                                .text_color(cx.theme().link),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(item.to_string()),
                        )
                }),
            )
            .child(
                div()
                    .w_full()
                    .rounded_xl()
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(gpui::hsla(0.0, 0.0, 0.5, 0.04))
                    .p_3()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Icon::new(IconName::GitHub)
                                            .with_size(px(16.0))
                                            .text_color(cx.theme().muted_foreground),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().link)
                                            .child(GITHUB_URL),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_1()
                                    .child(
                                        Clipboard::new("encourage-copy-github-url")
                                            .value(GITHUB_URL),
                                    )
                                    .child(
                                        Button::new("encourage-open-github")
                                            .icon(IconName::ExternalLink)
                                            .xsmall()
                                            .ghost()
                                            .on_click(|_: &ClickEvent, _, cx| {
                                                cx.open_url(GITHUB_URL);
                                            }),
                                    ),
                            ),
                    ),
            )
            .into_any_element();

        self.section_card(
            IconName::GitHub,
            t!("Encourage.alt_support_title").into(),
            body,
            cx,
        )
    }

    fn render_support_panel(&self, cx: &App) -> AnyElement {
        self.render_domestic_panel(cx)
    }

    fn render_domestic_panel(&self, cx: &App) -> AnyElement {
        let border = cx.theme().border;
        let background = cx.theme().background;
        let size = px(280.0);

        let wechat = self.render_payment_card(
            &t!("Encourage.wechat_label"),
            self.urls.wechat.clone(),
            self.offline_images.wechat.clone(),
            border,
            background,
            size,
            cx,
        );
        let alipay = self.render_payment_card(
            &t!("Encourage.alipay_label"),
            self.urls.alipay.clone(),
            self.offline_images.alipay.clone(),
            border,
            background,
            size,
            cx,
        );
        let paypal = self.render_payment_card(
            &t!("Encourage.paypal_label"),
            self.urls.paypal.clone(),
            self.offline_images.paypal.clone(),
            border,
            background,
            size,
            cx,
        );

        let mut cards = div().flex().flex_wrap().justify_center().w_full().gap_4();
        cards = cards.child(wechat).child(alipay).child(paypal);

        self.section_card(
            IconName::Heart,
            t!("Encourage.button_label").into(),
            cards.into_any_element(),
            cx,
        )
    }

    fn render_payment_card(
        &self,
        label: &str,
        online_url: Option<SharedString>,
        offline_image: Arc<Image>,
        border: gpui::Hsla,
        background: gpui::Hsla,
        size: gpui::Pixels,
        cx: &App,
    ) -> AnyElement {
        div()
            .w(px(330.0))
            .rounded_xl()
            .border_1()
            .border_color(border)
            .bg(gpui::hsla(0.0, 0.0, 0.5, 0.03))
            .p_4()
            .child(
                v_flex()
                    .w_full()
                    .gap_3()
                    .items_center()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Icon::new(IconName::Heart)
                                    .with_size(px(16.0))
                                    .text_color(cx.theme().link),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(label.to_string()),
                            ),
                    )
                    .child(Self::build_qr_image(
                        online_url,
                        offline_image,
                        border,
                        background,
                        size,
                    )),
            )
            .into_any_element()
    }

    fn build_qr_image(
        online_url: Option<SharedString>,
        offline_image: Arc<Image>,
        border: gpui::Hsla,
        background: gpui::Hsla,
        size: gpui::Pixels,
    ) -> AnyElement {
        let offline_for_fallback = offline_image.clone();
        let styled_offline = move |image: Arc<Image>| img(image).max_w(size).max_h(size);

        let frame = move |content: AnyElement| {
            div()
                .w(size)
                .h(size)
                .rounded_md()
                .border_1()
                .border_color(border)
                .bg(background)
                .overflow_hidden()
                .flex()
                .items_center()
                .justify_center()
                .child(content)
                .into_any_element()
        };

        match online_url {
            Some(url) => frame(
                img(url)
                    .max_w(size)
                    .max_h(size)
                    .with_fallback(move || {
                        styled_offline(offline_for_fallback.clone()).into_any_element()
                    })
                    .into_any_element(),
            ),
            None => frame(styled_offline(offline_image).into_any_element()),
        }
    }

    fn render_body(&self, cx: &App) -> AnyElement {
        v_flex()
            .w_full()
            .max_w(px(980.0))
            .gap_5()
            .child(self.render_intro(cx))
            .child(self.render_support_panel(cx))
            .child(self.render_alt_support_section(cx))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Encourage.footer_notice").to_string()),
            )
            .into_any_element()
    }

    fn render_settings_section(&self, cx: &App) -> AnyElement {
        v_flex()
            .w_full()
            .items_center()
            .p_4()
            .child(self.render_body(cx))
            .into_any_element()
    }
}

struct EncourageQrUrls {
    wechat: Option<SharedString>,
    alipay: Option<SharedString>,
    paypal: Option<SharedString>,
}

impl EncourageQrUrls {
    fn load() -> Self {
        Self {
            wechat: Self::read_url(WECHAT_QR_ENV, WECHAT_QR_BUILD),
            alipay: Self::read_url(ALIPAY_QR_ENV, ALIPAY_QR_BUILD),
            paypal: Self::read_url(PAYPAL_QR_ENV, PAYPAL_QR_BUILD),
        }
    }

    fn read_url(env_key: &str, build_time: Option<&'static str>) -> Option<SharedString> {
        if let Ok(value) = std::env::var(env_key) {
            if !value.trim().is_empty() {
                return Some(value.into());
            }
        }

        build_time
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.into())
    }
}

struct EncourageQrImages {
    wechat: Arc<Image>,
    alipay: Arc<Image>,
    paypal: Arc<Image>,
}

impl EncourageQrImages {
    fn load() -> Self {
        Self {
            wechat: Self::load_png(WECHAT_QR_OFFLINE),
            alipay: Self::load_png(ALIPAY_QR_OFFLINE),
            paypal: Self::load_png(PAYPAL_QR_OFFLINE),
        }
    }

    fn load_png(bytes: &'static [u8]) -> Arc<Image> {
        Arc::new(Image::from_bytes(ImageFormat::Png, bytes.to_vec()))
    }
}
