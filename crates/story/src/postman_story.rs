use gpui::{
    actions, div, prelude::FluentBuilder as _, rems, AnyElement, App, AppContext, Context, Entity,
    Focusable, InteractiveElement, IntoElement, ParentElement as _, Render,
    StatefulInteractiveElement as _, Styled, Task, WeakEntity, Window, WindowHandle,
};
use gpui_component::{
    button::Button,
    dropdown::{Dropdown, DropdownItem, DropdownState},
    h_flex,
    input::{InputState, TextInput},
    tab::{Tab, TabBar},
    v_flex, ActiveTheme as _, Disableable as _, Selectable as _,
};
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Method as ReqwestMethod,
};
use std::str::FromStr;

// Enum for identifying tabs
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TabId {
    Headers,
    QueryParams,
    Body,
}

#[derive(Clone, PartialEq, Debug)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    fn all() -> Vec<Self> {
        vec![
            HttpMethod::Get,
            HttpMethod::Post,
            HttpMethod::Put,
            HttpMethod::Delete,
            HttpMethod::Patch,
            HttpMethod::Head,
            HttpMethod::Options,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        }
    }
}

impl DropdownItem for HttpMethod {
    type Value = Self;

    fn title(&self) -> gpui::SharedString {
        self.as_str().into()
    }

    fn value(&self) -> &Self::Value {
        self
    }
}

// TODO: Define actions if needed for this story

// Define the main struct for our Postman-like feature
pub struct PostmanStory {
    url_input: Entity<InputState>,
    http_method_dropdown: Entity<DropdownState<Vec<HttpMethod>>>,
    active_tab: TabId,
    header_pairs: Vec<(Entity<InputState>, Entity<InputState>)>,
    query_param_pairs: Vec<(Entity<InputState>, Entity<InputState>)>,
    body_input: Entity<InputState>,
    client: Client,
    is_loading: bool,
    response_status: Option<String>,
    response_headers: Option<String>,
    response_body: Option<String>,
    error_message: Option<String>,
}

impl super::Story for PostmanStory {
    fn title() -> &'static str {
        "Postman"
    }

    fn description() -> &'static str {
        "A simple Postman-like HTTP request client."
    }

    fn closable() -> bool {
        true // Or false, depending on desired behavior
    }

    fn new_view(window: &mut Window, cx: &mut App) -> Entity<impl Render + Focusable> {
        Self::view(window, cx)
    }
}

impl PostmanStory {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx_self| Self::new(window, cx_self))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let url_input = cx.new(|cx_is| {
            InputState::new(window, cx_is)
                .default_value("https://api.github.com/repos/longbridge/gpui-component")
                .placeholder("Enter URL")
        });

        let http_method_dropdown =
            cx.new(|cx_ds| DropdownState::new(HttpMethod::all(), Some(0), window, cx_ds));

        let mut header_pairs = Vec::new();
        header_pairs.push(Self::create_empty_kv_pair(window, cx));

        let mut query_param_pairs = Vec::new();
        query_param_pairs.push(Self::create_empty_kv_pair(window, cx));

        let body_input = cx.new(|cx_is| {
            InputState::new(window, cx_is)
                .placeholder("Request body")
                .auto_grow(5, 20)
        });

        Self {
            url_input,
            http_method_dropdown,
            active_tab: TabId::Headers,
            header_pairs,
            query_param_pairs,
            body_input,
            client: Client::new(),
            is_loading: false,
            response_status: None,
            response_headers: None,
            response_body: None,
            error_message: None,
        }
    }

    fn create_empty_kv_pair(
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> (Entity<InputState>, Entity<InputState>) {
        let key_input = cx.new(|cx_is| InputState::new(window, cx_is).placeholder("Key"));
        let value_input = cx.new(|cx_is| InputState::new(window, cx_is).placeholder("Value"));
        (key_input, value_input)
    }
}

// Implement Focusable if you need to manage focus within this story
impl Focusable for PostmanStory {
    fn focus_handle(&self, cx: &gpui::App) -> gpui::FocusHandle {
        // For now, just focus the URL input as an example
        self.url_input.focus_handle(cx)
    }
}

impl Render for PostmanStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .id("postman-story")
            .size_full()
            .p_4()
            .gap_3()
            .child(
                h_flex()
                    .id("request_bar")
                    .w_full()
                    .gap_2()
                    .items_center()
                    .child(
                        Dropdown::new(&self.http_method_dropdown).width(rems(7.0)), // ~112px at 16px base
                    )
                    .child(TextInput::new(&self.url_input))
                    .child({
                        let label = if self.is_loading {
                            "Sending..."
                        } else {
                            "Send"
                        };
                        Button::new("send_button")
                            .label(label)
                            .disabled(self.is_loading)
                            .on_click(cx.listener(|this, _, _, cx_self| {
                                this.handle_send_request(cx_self);
                            }))
                    }),
            )
            .child(
                TabBar::new("request_config_tabs")
                    .child(
                        Tab::new("headers_tab")
                            .child("Headers")
                            .selected(self.active_tab == TabId::Headers)
                            .on_click(cx.listener(|this, _, _, cx_self| {
                                this.active_tab = TabId::Headers;
                                cx_self.notify();
                            })),
                    )
                    .child(
                        Tab::new("params_tab")
                            .child("Query Params")
                            .selected(self.active_tab == TabId::QueryParams)
                            .on_click(cx.listener(|this, _, _, cx_self| {
                                this.active_tab = TabId::QueryParams;
                                cx_self.notify();
                            })),
                    )
                    .child(
                        Tab::new("body_tab")
                            .child("Body")
                            .selected(self.active_tab == TabId::Body)
                            .on_click(cx.listener(|this, _, _, cx_self| {
                                this.active_tab = TabId::Body;
                                cx_self.notify();
                            })),
                    ),
            )
            .child(self.render_active_tab_content(cx))
            // Placeholder for future sections (headers, body, response)
            .child(self.render_response_area(cx))
    }
}

impl PostmanStory {
    fn handle_send_request(&mut self, cx: &mut Context<Self>) {
        if self.is_loading {
            return;
        }

        self.is_loading = true;
        self.response_status = None;
        self.response_headers = None;
        self.response_body = None;
        self.error_message = None;
        cx.notify();

        let url_str = self.url_input.read(cx).value().to_string();
        let selected_method = self
            .http_method_dropdown
            .read(cx)
            .selected_value()
            .cloned()
            .unwrap_or(HttpMethod::Get); // Default to GET

        let mut headers = HeaderMap::new();
        for (key_input, value_input) in &self.header_pairs {
            let key = key_input.read(cx).value().to_string();
            let value = value_input.read(cx).value().to_string();
            if !key.is_empty() {
                if let (Ok(header_name), Ok(header_value)) =
                    (HeaderName::from_str(&key), HeaderValue::from_str(&value))
                {
                    headers.insert(header_name, header_value);
                } else {
                    self.error_message = Some(format!("Invalid header: {} or {}", key, value));
                    self.is_loading = false;
                    cx.notify();
                    return;
                }
            }
        }

        let query_params_collected: Vec<(String, String)> = self
            .query_param_pairs
            .iter()
            .map(|(k, v)| {
                (
                    k.read(cx).value().to_string(),
                    v.read(cx).value().to_string(),
                )
            })
            .filter(|(k, _)| !k.is_empty())
            .collect();

        let final_url = if query_params_collected.is_empty() {
            url_str
        } else {
            match reqwest::Url::parse_with_params(&url_str, &query_params_collected) {
                Ok(url) => url.to_string(),
                Err(e) => {
                    self.error_message = Some(format!("Error building URL with params: {}", e));
                    self.is_loading = false;
                    cx.notify();
                    return;
                }
            }
        };

        let body_str = self.body_input.read(cx).value().to_string();
        let client = self.client.clone(); // Clone client for the async task
        let view_handle: WeakEntity<Self> = cx.entity().downgrade(); // Get a weak handle to the view

        cx.spawn(async move |window, cx| {
            let req_method =
                ReqwestMethod::from_str(selected_method.as_str()).unwrap_or(ReqwestMethod::GET);

            let mut request_builder = client.request(req_method, &final_url).headers(headers);
            if selected_method != HttpMethod::Get
                && selected_method != HttpMethod::Head
                && !body_str.is_empty()
            {
                request_builder = request_builder.body(body_str);
            }

            match request_builder.send().await {
                Ok(response) => {
                    let status = response.status().to_string();
                    let headers_string = format!("{:#?}", response.headers()); // Simple debug format
                    let response_body_result: Result<String, reqwest::Error> =
                        response.text().await;

                    let _ = cx.update(|cx| {
                        view_handle.update(cx, |this, cx| {
                            this.response_status = Some(status);
                            this.response_headers = Some(headers_string);
                            match response_body_result {
                                Ok(text) => {
                                    this.response_body = Some(text);
                                }
                                Err(e) => {
                                    // Error reading body, but we still got status/headers
                                    this.error_message =
                                        Some(format!("Error reading response body: {}", e));
                                    this.response_body = None; // Or Some("Error: Could not read body.".to_string())
                                }
                            }
                            this.is_loading = false;
                            cx.notify();
                        })
                    });
                }
                Err(err) => {
                    let _ = view_handle.update(cx, |this, cx_update| {
                        this.error_message = Some(format!("Request failed: {}", err));
                        this.is_loading = false;
                        cx_update.notify();
                    });
                }
            }
        })
        .detach();
    }

    fn render_kv_pair_inputs(
        key_input: &Entity<InputState>,
        value_input: &Entity<InputState>,
    ) -> Vec<AnyElement> {
        vec![
            div()
                .flex_grow()
                .child(TextInput::new(key_input))
                .into_any_element(),
            div()
                .flex_grow()
                .child(TextInput::new(value_input))
                .into_any_element(),
        ]
    }

    fn render_response_area(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme(); // Get theme for colors

        if self.is_loading {
            // TODO: Use Indicator component if available and simple
            return div()
                .p_4()
                .size_full()
                .flex()
                .justify_center()
                .items_center()
                .child("Loading...")
                .into_any_element();
        }

        if let Some(err_msg) = &self.error_message {
            return div()
                .p_4()
                .text_color(theme.danger) // Use theme color for error
                .child(format!("Error: {}", err_msg))
                .into_any_element();
        }

        if self.response_status.is_none() && self.response_body.is_none() {
            return div()
                .p_4()
                .child("Send a request to see the response.")
                .into_any_element();
        }

        v_flex()
            .id("response-display")
            .p_2()
            .gap_3() // Increased gap for sections
            .flex_grow() // Ensure it takes available space
            .child(div().child(format!(
                "Status: {}",
                self.response_status.as_deref().unwrap_or("N/A")
            )))
            .child(
                v_flex()
                    .id("response-headers-section")
                    .gap_1()
                    .child(div().font_weight(gpui::FontWeight::BOLD).child("Headers:"))
                    .child(
                        div()
                            .id("response-body")
                            .max_h(rems(10.0)) // Max height for scroll, e.g., 10 rems (160px)
                            .overflow_y_scroll()
                            .p_1()
                            .border_1()
                            .border_color(theme.border)
                            .children(
                                self.response_headers
                                    .clone()
                                    .unwrap_or("N/A".to_string())
                                    .lines()
                                    .map(|line| div().child(line.to_string()))
                                    .collect::<Vec<_>>(),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .id("response-body-section")
                    .gap_1()
                    .flex_grow() // Allow body to take more space
                    .child(div().font_weight(gpui::FontWeight::BOLD).child("Body:"))
                    .child(
                        div()
                            .id("body")
                            .flex_grow() // Ensure this div itself can grow
                            .max_h(rems(20.0)) // Max height for scroll, e.g., 20 rems (320px)
                            .overflow_y_scroll()
                            .p_1()
                            .border_1()
                            .border_color(theme.border)
                            .child(self.response_body.clone().unwrap_or("N/A".into())),
                    ),
            )
            .into_any_element()
    }

    fn render_active_tab_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.active_tab {
            TabId::Headers => {
                let header_rows = self
                    .header_pairs
                    .iter()
                    .enumerate()
                    .map(|(index, (key_input, value_input))| {
                        h_flex()
                            .id(("header_pair", index))
                            .gap_2()
                            .items_center() // Align items vertically
                            .children(Self::render_kv_pair_inputs(key_input, value_input))
                            .child(
                                Button::new(("remove_header", index))
                                    .label("Remove")
                                    .on_click(cx.listener(move |this, _, _, cx_self| {
                                        this.header_pairs.remove(index);
                                        cx_self.notify();
                                    })),
                            )
                    })
                    .collect::<Vec<_>>();

                v_flex()
                    .id("headers_content")
                    .p_2()
                    .gap_2()
                    .children(header_rows)
                    .child(
                        Button::new("add_header")
                            .label("Add Header")
                            .on_click(cx.listener(|this, _, window, cx| {
                                let new_pair = Self::create_empty_kv_pair(window, cx);
                                this.header_pairs.push(new_pair);
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            }
            TabId::QueryParams => {
                let param_rows = self
                    .query_param_pairs
                    .iter()
                    .enumerate()
                    .map(|(index, (key_input, value_input))| {
                        h_flex()
                            .id(("query_param_pair", index))
                            .gap_2()
                            .items_center() // Align items vertically
                            .children(Self::render_kv_pair_inputs(key_input, value_input))
                            .child(
                                Button::new(("remove_query_param", index))
                                    .label("Remove")
                                    .on_click(cx.listener(move |this, _, _, cx_self| {
                                        this.query_param_pairs.remove(index);
                                        cx_self.notify();
                                    })),
                            )
                    })
                    .collect::<Vec<_>>();

                v_flex()
                    .id("query_params_content")
                    .p_2()
                    .gap_2()
                    .children(param_rows)
                    .child(
                        Button::new("add_param")
                            .label("Add Param")
                            .on_click(cx.listener(|this, _, window, cx| {
                                let new_pair = Self::create_empty_kv_pair(window, cx);
                                this.query_param_pairs.push(new_pair);
                                cx.notify();
                            })),
                    )
                    .into_any_element()
            }
            TabId::Body => v_flex()
                .id("body_content")
                .p_2()
                .child(div().flex_grow().child(TextInput::new(&self.body_input)))
                .into_any_element(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HttpMethod;

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
        assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
        assert_eq!(HttpMethod::Head.as_str(), "HEAD");
        assert_eq!(HttpMethod::Options.as_str(), "OPTIONS");
    }

    #[test]
    fn test_http_method_all() {
        let all_methods = HttpMethod::all();
        assert_eq!(all_methods.len(), 7); // Assuming 7 methods defined
        assert!(all_methods.contains(&HttpMethod::Get));
        assert!(all_methods.contains(&HttpMethod::Post));
        assert!(all_methods.contains(&HttpMethod::Put));
        assert!(all_methods.contains(&HttpMethod::Delete));
        assert!(all_methods.contains(&HttpMethod::Patch));
        assert!(all_methods.contains(&HttpMethod::Head));
        assert!(all_methods.contains(&HttpMethod::Options));
    }
}
