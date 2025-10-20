import Link from "next/link";
import {
  ArrowRight,
  Code2,
  Zap,
  Palette,
  Layout,
  BarChart3,
  Terminal,
  Blocks,
  SquareCode,
} from "lucide-react";
import { DynamicCodeBlock } from "fumadocs-ui/components/dynamic-codeblock";

export default function HomePage() {
  return (
    <main className="flex flex-1 flex-col">
      {/* Hero Section */}
      <section className="container flex flex-col items-center justify-center gap-8 py-24 text-center md:py-32">
        <div className="flex flex-col gap-4">
          <h1 className="text-4xl font-bold tracking-tight sm:text-5xl md:text-6xl lg:text-7xl">
            GPUI Component
          </h1>
          <p className="text-xl text-fd-muted-foreground max-w-[42rem] leading-normal sm:text-2xl sm:leading-8">
            Rust GUI components for building fantastic cross-platform desktop
            application by using{" "}
            <a
              href="https://gpui.rs"
              target="_blank"
              rel="noopener noreferrer"
              className="font-semibold text-fd-foreground underline underline-offset-4 hover:text-fd-primary"
            >
              GPUI
            </a>
          </p>
        </div>

        <div className="flex flex-col gap-4 sm:flex-row">
          <Link
            href="/docs"
            className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-8 py-3 text-sm font-medium text-fd-primary-foreground shadow transition-colors hover:bg-fd-primary/90 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-fd-ring disabled:pointer-events-none disabled:opacity-50"
          >
            Get Started
            <ArrowRight className="ml-2 h-4 w-4" />
          </Link>
          <Link
            href="https://github.com/longbridge/gpui-component"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center justify-center rounded-lg border border-fd-border bg-fd-background px-8 py-3 text-sm font-medium shadow-sm transition-colors hover:bg-fd-accent hover:text-fd-accent-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-fd-ring disabled:pointer-events-none disabled:opacity-50"
          >
            View on GitHub
          </Link>
        </div>
      </section>

      {/* Features Section */}
      <section className="container py-24 md:py-32">
        <div className="grid gap-12 lg:grid-cols-3">
          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-green-500 dark:bg-green-700">
              <Blocks className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">40+ Components</h3>
            <p className="text-fd-muted-foreground">
              Comprehensive library of cross-platform desktop UI components for
              building feature-rich applications.
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-blue-500 dark:bg-blue-700">
              <Zap className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">High Performance</h3>
            <p className="text-fd-muted-foreground">
              Virtualized Table and List components for smooth rendering of
              large datasets with minimal memory footprint.
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-red-500 dark:bg-red-700">
              <Palette className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">Themeable</h3>
            <p className="text-fd-muted-foreground">
              Built-in Theme system with customizable colors, supporting
              multiple themes and dark mode out of the box.
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-yellow-500 dark:bg-yellow-700">
              <Layout className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">Flexible Layouts</h3>
            <p className="text-fd-muted-foreground">
              Dock layout for panel arrangements, resizable panels, and freeform
              layouts for any application structure.
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-purple-500 dark:bg-purple-700">
              <BarChart3 className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">Data Visualization</h3>
            <p className="text-fd-muted-foreground">
              Built-in chart components for visualizing data with Line, Bar,
              Area, and Pie charts.
            </p>
          </div>

          <div className="flex flex-col gap-4">
            <div className="flex h-12 w-12 items-center justify-center rounded-lg bg-cyan-500 dark:bg-cyan-700">
              <SquareCode className="h-6 w-6 text-white" />
            </div>
            <h3 className="text-xl font-bold">Code Editor</h3>
            <p className="text-fd-muted-foreground">
              High-performance code editor with LSP support, syntax
              highlighting, powered by{" "}
              <a href="https://tree-sitter.github.io/" target="_blank">
                Tree Sitter
              </a>{" "}
              and{" "}
              <a href="https://github.com/cessen/ropey" target="_blank">
                Rope
              </a>
              .
            </p>
          </div>
        </div>
      </section>

      <section className="container py-24 md:py-32">
        <div className="flex flex-col gap-8">
          <div className="flex flex-col gap-4 text-center">
            <h2 className="text-3xl font-bold">Simple and Intuitive API</h2>
            <p className="text-fd-muted-foreground text-lg max-w-[42rem] mx-auto">
              Get started with just a few lines of code. Stateless components
              make it easy to build complex UIs.
            </p>
          </div>

          <DynamicCodeBlock
            lang="rust"
            code={`use gpui::*;
use gpui_component::{button::*, *};

pub struct HelloWorld;
impl Render for HelloWorld {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .size_full()
            .items_center()
            .justify_center()
            .child("Hello, World!")
            .child(
                Button::new("ok")
                    .primary()
                    .label("Let's Go!")
                    .on_click(|_, _, _| println!("Clicked!")),
            )
    }
}

fn main() {
    let app = Application::new();

    app.run(move |cx| {
        // This must be called before using any GPUI Component features.
        gpui_component::init(cx);

        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|_| HelloWorld);
                // This first level on the window, should be a Root.
                cx.new(|cx| Root::new(view.into(), window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}`}
          ></DynamicCodeBlock>
        </div>
      </section>

      {/* CTA Section */}
      <section className="container py-24 md:py-32">
        <div className="flex flex-col gap-8 rounded-lg border bg-fd-muted/50 p-12 text-center">
          <div className="flex flex-col gap-4">
            <h2 className="text-3xl font-bold">Ready to get started?</h2>
            <p className="text-fd-muted-foreground text-lg max-w-[42rem] mx-auto">
              Explore our comprehensive documentation and start building amazing
              desktop applications today.
            </p>
          </div>

          <div className="flex flex-col gap-4 sm:flex-row justify-center">
            <Link
              href="/docs"
              className="inline-flex items-center justify-center rounded-lg bg-fd-primary px-8 py-3 text-sm font-medium text-fd-primary-foreground shadow transition-colors hover:bg-fd-primary/90"
            >
              Read Documentation
              <ArrowRight className="ml-2 h-4 w-4" />
            </Link>
            <Link
              href="/docs/getting-started"
              className="inline-flex items-center justify-center rounded-lg border border-fd-border bg-fd-background px-8 py-3 text-sm font-medium shadow-sm transition-colors hover:bg-fd-accent hover:text-fd-accent-foreground"
            >
              Quick Start Guide
            </Link>
          </div>
        </div>
      </section>
    </main>
  );
}
