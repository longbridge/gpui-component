// ============================================================================
// 鼓励作者对话框
// ============================================================================

use gpui::{
    AnyElement, App, Context, FocusHandle, Focusable, FontWeight, Image, ImageFormat, IntoElement,
    ParentElement, Render, SharedString, Styled, StyledImage, Window, div, img, px,
};
use gpui_component::{ActiveTheme, h_flex, v_flex};
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

pub struct EncourageDialog {
    focus_handle: FocusHandle,
    urls: EncourageQrUrls,
    offline_images: EncourageQrImages,
}

impl EncourageDialog {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            urls: EncourageQrUrls::load(),
            offline_images: EncourageQrImages::load(),
        }
    }

    fn render_intro(&self, _cx: &mut Context<Self>) -> AnyElement {
        v_flex()
            .w_full()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .child(t!("Encourage.intro_line1").to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .child(t!("Encourage.intro_line2").to_string()),
            )
            .child(
                div()
                    .text_sm()
                    .child(t!("Encourage.intro_line3").to_string()),
            )
            .into_any_element()
    }

    fn render_support_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        self.render_domestic_panel(cx)
    }

    fn render_domestic_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        let border = cx.theme().border;
        let background = cx.theme().background;
        let size = px(180.0);

        let wechat = self.render_payment_card(
            &t!("Encourage.wechat_label"),
            self.urls.wechat.clone(),
            self.offline_images.wechat.clone(),
            border,
            background,
            size,
        );
        let alipay = self.render_payment_card(
            &t!("Encourage.alipay_label"),
            self.urls.alipay.clone(),
            self.offline_images.alipay.clone(),
            border,
            background,
            size,
        );

        let paypal = self.render_payment_card(
            &t!("Encourage.paypal_label"),
            self.urls.paypal.clone(),
            self.offline_images.paypal.clone(),
            border,
            background,
            size,
        );

        v_flex()
            .w_full()
            .gap_3()
            .child(
                h_flex()
                    .w_full()
                    .justify_center()
                    .gap_6()
                    .child(wechat)
                    .child(alipay)
                    .child(paypal),
            )
            .into_any_element()
    }

    fn render_payment_card(
        &self,
        label: &str,
        online_url: Option<SharedString>,
        offline_image: Arc<Image>,
        border: gpui::Hsla,
        background: gpui::Hsla,
        size: gpui::Pixels,
    ) -> AnyElement {
        v_flex()
            .w(size)
            .gap_2()
            .items_center()
            .flex_shrink_0()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(label.to_string()),
            )
            .child(Self::build_qr_image(
                online_url,
                offline_image,
                border,
                background,
                size,
            ))
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
}

impl Focusable for EncourageDialog {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for EncourageDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_4()
            .child(self.render_intro(cx))
            .child(self.render_support_panel(cx))
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight::MEDIUM)
                            .child(t!("Encourage.alt_support_title").to_string()),
                    )
                    .children(
                        [
                            t!("Encourage.alt_support_item1"),
                            t!("Encourage.alt_support_item2"),
                            t!("Encourage.alt_support_item3"),
                        ]
                        .into_iter()
                        .map(|item| {
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("• {}", item))
                        }),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Encourage.footer_notice").to_string()),
            )
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
