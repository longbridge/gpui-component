use std::{
    io::{Read as _, Write as _},
    net::TcpListener,
    ops::Deref,
    thread,
};

use gpui::{Entity, IntoElement as _, TestAppContext, VisualTestContext};

use crate::{Capabilities, ScriptView, ShellRuntime};

const POST_PROBE: &str = r#"
import { View, v_flex, text, spawn, with_cx } from "gpui";

export default class Probe extends View {
  init() {
    this.state = "pending";
    spawn(async () => {
      try {
        const response = await fetch("__URL__", {
          method: "POST",
          headers: {
            "Accept": "application/json",
            "Authorization": "Bearer access-token",
            "Content-Type": "application/x-www-form-urlencoded",
          },
          body: "grant_type=refresh_token&refresh_token=refresh-token",
        });
        this.state = `${response.status}|${await response.text()}`;
      } catch (error) {
        this.state = `rejected:${error.message}`;
      }
      with_cx((cx) => cx.notify());
    });
  }
  render() { return v_flex().child(text(this.state)); }
}
"#;

/// A regression here would silently turn OAuth token exchanges into GETs, or
/// drop bearer credentials before they leave the capability-gated boundary.
#[gpui::test]
fn fetch_posts_string_bodies_with_custom_headers(cx: &mut TestAppContext) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("HTTP listener");
    let address = listener.local_addr().expect("listener address");
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("HTTP connection");
        let request = read_request(&mut stream);
        assert!(request.starts_with("POST /token HTTP/1.1\r\n"), "{request}");
        assert!(
            request.contains("accept: application/json\r\n"),
            "{request}"
        );
        assert!(
            request.contains("authorization: Bearer access-token\r\n"),
            "{request}"
        );
        assert!(
            request.contains("content-type: application/x-www-form-urlencoded\r\n"),
            "{request}"
        );
        assert!(
            request.ends_with("grant_type=refresh_token&refresh_token=refresh-token"),
            "{request}"
        );
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
            .expect("HTTP response");
    });

    let source = POST_PROBE.replace("__URL__", &format!("http://{address}/token"));
    let (_runtime, view, mut context) = probe(cx, &source);
    context.run_until_parked();
    draw(&mut context, &view);
    let rendered = snapshot(&mut context, &view);
    assert!(rendered.contains("200|ok"), "{rendered}");
    server.join().expect("HTTP server");
}

fn read_request(stream: &mut std::net::TcpStream) -> String {
    let mut request = Vec::new();
    let mut buffer = [0; 1024];
    let header_end = loop {
        let read = stream.read(&mut buffer).expect("HTTP request");
        assert_ne!(read, 0, "connection closed before HTTP headers");
        request.extend_from_slice(&buffer[..read]);
        if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
            break end + 4;
        }
    };
    let headers = String::from_utf8_lossy(&request[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| line.strip_prefix("content-length: "))
        .expect("content length")
        .parse::<usize>()
        .expect("numeric content length");
    while request.len() < header_end + content_length {
        let read = stream.read(&mut buffer).expect("HTTP request body");
        assert_ne!(read, 0, "connection closed before HTTP request body");
        request.extend_from_slice(&buffer[..read]);
    }
    String::from_utf8(request).expect("UTF-8 request")
}

fn probe(
    cx: &mut TestAppContext,
    source: &str,
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    probe_with_hosts(cx, source, ["127.0.0.1"])
}

fn probe_with_hosts<const N: usize>(
    cx: &mut TestAppContext,
    source: &str,
    hosts: [&str; N],
) -> (
    std::rc::Rc<ShellRuntime>,
    Entity<ScriptView>,
    VisualTestContext,
) {
    cx.update(crate::init);
    crate::set_capabilities(Capabilities::new().network_hosts(hosts.map(str::to_owned)));
    let runtime = ShellRuntime::new().expect("runtime");
    runtime.use_direct_http_for_tests();
    cx.update(|cx| runtime.set_global(cx));
    let view_type = runtime
        .load_source("http-request.js", source)
        .expect("load");
    let window = cx.add_window(|_, _| Empty);
    let mut context = VisualTestContext::from_window(*window.deref(), cx);
    let view = context
        .update(|window, cx| runtime.instantiate_view(&view_type, window, cx))
        .expect("instantiate");
    (runtime, view, context)
}

fn draw(context: &mut VisualTestContext, view: &Entity<ScriptView>) {
    let view = view.clone();
    context.draw(
        gpui::Point::default(),
        gpui::size(gpui::px(400.), gpui::px(300.)),
        move |_, _| view.into_any_element(),
    );
}

fn snapshot(context: &mut VisualTestContext, view: &Entity<ScriptView>) -> String {
    context.update(|_, cx| {
        view.read(cx)
            .snapshot()
            .map(crate::RenderSnapshot::debug_tree)
            .unwrap_or_default()
    })
}

struct Empty;

impl gpui::Render for Empty {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        gpui::div()
    }
}
