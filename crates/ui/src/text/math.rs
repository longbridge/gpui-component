use std::sync::Arc;

use gpui::{Hsla, Image, ImageFormat, Rgba};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MathDisplay {
    #[allow(dead_code)]
    Inline,
    Block,
}

pub(crate) fn render_math_image(
    source: &str,
    display: MathDisplay,
    font_size: f32,
    color: Hsla,
) -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        render_math_svg(source, display, font_size, color).into_bytes(),
    ))
}

pub(crate) fn render_math_svg(
    source: &str,
    display: MathDisplay,
    font_size: f32,
    color: Hsla,
) -> String {
    let font_size = match display {
        MathDisplay::Inline => font_size.max(10.0),
        MathDisplay::Block => (font_size * 1.18).max(12.0),
    };
    let expr = Parser::new(source).parse();
    let layout = layout_expr(&expr, font_size);
    let margin_x = match display {
        MathDisplay::Inline => font_size * 0.12,
        MathDisplay::Block => font_size * 0.3,
    };
    let margin_y = match display {
        MathDisplay::Inline => font_size * 0.08,
        MathDisplay::Block => font_size * 0.22,
    };
    let width = (layout.width + margin_x * 2.0).ceil().max(1.0);
    let height = (layout.ascent + layout.descent + margin_y * 2.0)
        .ceil()
        .max(1.0);
    let baseline = margin_y + layout.ascent;
    let (fill, opacity) = svg_color(color);

    let mut svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width:.1}" height="{height:.1}" viewBox="0 0 {width:.1} {height:.1}">"#
    );
    svg.push_str(&format!(
        r#"<g fill="{fill}" fill-opacity="{opacity:.3}" font-family="Times New Roman, STIX Two Math, Cambria Math, serif">"#
    ));

    for line in &layout.lines {
        svg.push_str(&format!(
            r#"<line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{fill}" stroke-opacity="{opacity:.3}" stroke-width="{:.2}" stroke-linecap="round"/>"#,
            margin_x + line.x1,
            baseline + line.y1,
            margin_x + line.x2,
            baseline + line.y2,
            line.stroke_width
        ));
    }

    for text in &layout.texts {
        svg.push_str(&format!(
            r#"<text x="{:.2}" y="{:.2}" font-size="{:.2}"{}>{}</text>"#,
            margin_x + text.x,
            baseline + text.y,
            text.size,
            if text.italic {
                r#" font-style="italic""#
            } else {
                ""
            },
            escape_xml(&text.text)
        ));
    }

    svg.push_str("</g></svg>");
    svg
}

#[derive(Debug, Clone, PartialEq)]
enum Expr {
    Row(Vec<Expr>),
    Text(String),
    SupSub {
        base: Box<Expr>,
        sup: Option<Box<Expr>>,
        sub: Option<Box<Expr>>,
    },
    Fraction {
        numerator: Box<Expr>,
        denominator: Box<Expr>,
    },
    Sqrt(Box<Expr>),
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
        }
    }

    fn parse(mut self) -> Expr {
        self.parse_row(None)
    }

    fn parse_row(&mut self, until: Option<char>) -> Expr {
        let mut items = Vec::new();

        while let Some(ch) = self.peek() {
            if Some(ch) == until {
                self.pos += 1;
                break;
            }

            match ch {
                '^' | '_' => {
                    self.pos += 1;
                    let script = self.parse_script();
                    if let Some(base) = items.pop() {
                        items.push(apply_script(base, ch == '^', script));
                    } else {
                        items.push(Expr::Text(ch.to_string()));
                    }
                }
                '}' if until.is_none() => {
                    items.push(Expr::Text(ch.to_string()));
                    self.pos += 1;
                }
                _ => items.push(self.parse_atom()),
            }
        }

        compact_row(items)
    }

    fn parse_atom(&mut self) -> Expr {
        match self.next() {
            Some('{') => self.parse_row(Some('}')),
            Some('\\') => self.parse_command(),
            Some(ch) => Expr::Text(ch.to_string()),
            None => Expr::Text(String::new()),
        }
    }

    fn parse_script(&mut self) -> Expr {
        self.skip_spaces();
        match self.peek() {
            Some('{') => {
                self.pos += 1;
                self.parse_row(Some('}'))
            }
            Some('\\') => {
                self.pos += 1;
                self.parse_command()
            }
            Some(_) => self.parse_atom(),
            None => Expr::Text(String::new()),
        }
    }

    fn parse_required_group(&mut self) -> Expr {
        self.skip_spaces();
        if self.peek() == Some('{') {
            self.pos += 1;
            self.parse_row(Some('}'))
        } else {
            self.parse_script()
        }
    }

    fn parse_command(&mut self) -> Expr {
        let command = self.consume_command();
        match command.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let numerator = self.parse_required_group();
                let denominator = self.parse_required_group();
                Expr::Fraction {
                    numerator: Box::new(numerator),
                    denominator: Box::new(denominator),
                }
            }
            "sqrt" => {
                if self.peek() == Some('[') {
                    self.skip_bracket_group();
                }
                Expr::Sqrt(Box::new(self.parse_required_group()))
            }
            "text" | "mathrm" | "operatorname" => {
                Expr::Text(flatten_text(&self.parse_required_group()))
            }
            "left" | "right" => {
                self.skip_spaces();
                match self.next() {
                    Some('.') => Expr::Text(String::new()),
                    Some(ch) => Expr::Text(delimiter_symbol(ch).to_string()),
                    None => Expr::Text(String::new()),
                }
            }
            "," | ":" => Expr::Text(" ".to_string()),
            ";" => Expr::Text("  ".to_string()),
            "!" => Expr::Text(String::new()),
            _ => Expr::Text(
                command_symbol(&command)
                    .map(str::to_string)
                    .unwrap_or_else(|| command),
            ),
        }
    }

    fn consume_command(&mut self) -> String {
        let start = self.pos;
        while matches!(self.peek(), Some(ch) if ch.is_ascii_alphabetic()) {
            self.pos += 1;
        }

        if self.pos > start {
            self.chars[start..self.pos].iter().collect()
        } else {
            self.next().map(|ch| ch.to_string()).unwrap_or_default()
        }
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(ch) if ch.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn skip_bracket_group(&mut self) {
        if self.next() != Some('[') {
            return;
        }

        let mut depth = 1;
        while let Some(ch) = self.next() {
            match ch {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += 1;
        Some(ch)
    }
}

fn compact_row(items: Vec<Expr>) -> Expr {
    let mut compact = Vec::new();
    for item in items {
        match item {
            Expr::Text(text) if text.is_empty() => {}
            Expr::Row(children) if children.is_empty() => {}
            Expr::Text(text) => {
                if let Some(Expr::Text(prev)) = compact.last_mut() {
                    prev.push_str(&text);
                } else {
                    compact.push(Expr::Text(text));
                }
            }
            item => compact.push(item),
        }
    }

    if compact.len() == 1 {
        compact.pop().unwrap()
    } else {
        Expr::Row(compact)
    }
}

fn apply_script(base: Expr, is_sup: bool, script: Expr) -> Expr {
    match base {
        Expr::SupSub { base, sup, sub } => {
            if is_sup {
                Expr::SupSub {
                    base,
                    sup: Some(Box::new(script)),
                    sub,
                }
            } else {
                Expr::SupSub {
                    base,
                    sup,
                    sub: Some(Box::new(script)),
                }
            }
        }
        base => {
            if is_sup {
                Expr::SupSub {
                    base: Box::new(base),
                    sup: Some(Box::new(script)),
                    sub: None,
                }
            } else {
                Expr::SupSub {
                    base: Box::new(base),
                    sup: None,
                    sub: Some(Box::new(script)),
                }
            }
        }
    }
}

#[derive(Default)]
struct Layout {
    width: f32,
    ascent: f32,
    descent: f32,
    texts: Vec<TextRun>,
    lines: Vec<LineRun>,
}

struct TextRun {
    x: f32,
    y: f32,
    size: f32,
    text: String,
    italic: bool,
}

struct LineRun {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    stroke_width: f32,
}

fn layout_expr(expr: &Expr, size: f32) -> Layout {
    match expr {
        Expr::Row(items) => layout_row(items, size),
        Expr::Text(text) => layout_text(text, size),
        Expr::SupSub { base, sup, sub } => {
            layout_sup_sub(base, sup.as_deref(), sub.as_deref(), size)
        }
        Expr::Fraction {
            numerator,
            denominator,
        } => layout_fraction(numerator, denominator, size),
        Expr::Sqrt(body) => layout_sqrt(body, size),
    }
}

fn layout_row(items: &[Expr], size: f32) -> Layout {
    let mut out = Layout::default();
    out.ascent = size * 0.8;
    out.descent = size * 0.25;

    let mut x = 0.0;
    for item in items {
        let mut child = layout_expr(item, size);
        offset_layout(&mut child, x, 0.0);
        out.ascent = out.ascent.max(child.ascent);
        out.descent = out.descent.max(child.descent);
        x += child.width;
        out.texts.extend(child.texts);
        out.lines.extend(child.lines);
    }

    out.width = x.max(size * 0.2);
    out
}

fn layout_text(text: &str, size: f32) -> Layout {
    let display = normalize_text(text);
    let width = display.chars().map(|ch| char_width(ch, size)).sum::<f32>();
    Layout {
        width: width.max(size * 0.2),
        ascent: size * 0.8,
        descent: size * 0.25,
        texts: vec![TextRun {
            x: 0.0,
            y: 0.0,
            size,
            italic: should_italicize(&display),
            text: display,
        }],
        lines: Vec::new(),
    }
}

fn layout_sup_sub(base: &Expr, sup: Option<&Expr>, sub: Option<&Expr>, size: f32) -> Layout {
    let mut out = Layout::default();
    let mut base_layout = layout_expr(base, size);
    let script_size = size * 0.66;
    let script_x = base_layout.width + size * 0.06;
    let mut width = base_layout.width;
    out.ascent = base_layout.ascent;
    out.descent = base_layout.descent;
    out.texts.append(&mut base_layout.texts);
    out.lines.append(&mut base_layout.lines);

    if let Some(sup) = sup {
        let mut sup_layout = layout_expr(sup, script_size);
        let y = -base_layout.ascent * 0.55;
        out.ascent = out.ascent.max(-y + sup_layout.ascent);
        offset_layout(&mut sup_layout, script_x, y);
        width = width.max(script_x + sup_layout.width);
        out.texts.extend(sup_layout.texts);
        out.lines.extend(sup_layout.lines);
    }

    if let Some(sub) = sub {
        let mut sub_layout = layout_expr(sub, script_size);
        let y = base_layout.descent + sub_layout.ascent * 0.5;
        out.descent = out.descent.max(y + sub_layout.descent);
        offset_layout(&mut sub_layout, script_x, y);
        width = width.max(script_x + sub_layout.width);
        out.texts.extend(sub_layout.texts);
        out.lines.extend(sub_layout.lines);
    }

    out.width = width;
    out
}

fn layout_fraction(numerator: &Expr, denominator: &Expr, size: f32) -> Layout {
    let script_size = size * 0.82;
    let mut numerator = layout_expr(numerator, script_size);
    let mut denominator = layout_expr(denominator, script_size);
    let pad = size * 0.28;
    let gap = size * 0.18;
    let width = numerator.width.max(denominator.width) + pad * 2.0;

    let num_x = (width - numerator.width) / 2.0;
    let den_x = (width - denominator.width) / 2.0;
    let num_y = -gap - numerator.descent;
    let den_y = gap + denominator.ascent;

    offset_layout(&mut numerator, num_x, num_y);
    offset_layout(&mut denominator, den_x, den_y);

    let mut out = Layout {
        width,
        ascent: -num_y + numerator.ascent,
        descent: den_y + denominator.descent,
        texts: numerator.texts,
        lines: numerator.lines,
    };
    out.texts.extend(denominator.texts);
    out.lines.extend(denominator.lines);
    out.lines.push(LineRun {
        x1: pad * 0.45,
        y1: 0.0,
        x2: width - pad * 0.45,
        y2: 0.0,
        stroke_width: (size * 0.055).max(1.0),
    });
    out
}

fn layout_sqrt(body: &Expr, size: f32) -> Layout {
    let mut root = layout_text("\u{221a}", size * 1.18);
    let mut body = layout_expr(body, size);
    let gap = size * 0.08;
    let body_x = root.width + gap;
    let overline_y = -body.ascent - size * 0.05;
    let mut out = Layout::default();

    offset_layout(&mut body, body_x, 0.0);
    out.ascent = root.ascent.max(body.ascent + size * 0.12);
    out.descent = root.descent.max(body.descent);
    out.width = body_x + body.width + gap;
    out.texts.append(&mut root.texts);
    out.texts.extend(body.texts);
    out.lines.extend(body.lines);
    out.lines.push(LineRun {
        x1: body_x - gap * 0.25,
        y1: overline_y,
        x2: out.width,
        y2: overline_y,
        stroke_width: (size * 0.05).max(1.0),
    });
    out
}

fn offset_layout(layout: &mut Layout, dx: f32, dy: f32) {
    for text in &mut layout.texts {
        text.x += dx;
        text.y += dy;
    }
    for line in &mut layout.lines {
        line.x1 += dx;
        line.x2 += dx;
        line.y1 += dy;
        line.y2 += dy;
    }
}

fn normalize_text(text: &str) -> String {
    text.replace('-', "\u{2212}")
}

fn should_italicize(text: &str) -> bool {
    text.chars().any(|ch| ch.is_ascii_alphabetic())
        && text
            .chars()
            .all(|ch| ch.is_ascii_alphabetic() || ch.is_ascii_digit())
}

fn char_width(ch: char, size: f32) -> f32 {
    if ch.is_whitespace() {
        size * 0.34
    } else if ch.is_ascii_digit() {
        size * 0.52
    } else if ch.is_ascii_alphabetic() {
        size * 0.56
    } else if matches!(
        ch,
        '+' | '=' | '\u{2212}' | '\u{00b1}' | '\u{00d7}' | '\u{00f7}'
    ) {
        size * 0.72
    } else {
        size * 0.68
    }
}

fn flatten_text(expr: &Expr) -> String {
    match expr {
        Expr::Row(items) => items.iter().map(flatten_text).collect(),
        Expr::Text(text) => text.clone(),
        Expr::SupSub { base, sup, sub } => {
            let mut out = flatten_text(base);
            if let Some(sup) = sup {
                out.push('^');
                out.push_str(&flatten_text(sup));
            }
            if let Some(sub) = sub {
                out.push('_');
                out.push_str(&flatten_text(sub));
            }
            out
        }
        Expr::Fraction {
            numerator,
            denominator,
        } => format!(
            "({})/({})",
            flatten_text(numerator),
            flatten_text(denominator)
        ),
        Expr::Sqrt(body) => format!("\u{221a}{}", flatten_text(body)),
    }
}

fn delimiter_symbol(ch: char) -> &'static str {
    match ch {
        '(' => "(",
        ')' => ")",
        '[' => "[",
        ']' => "]",
        '{' => "{",
        '}' => "}",
        '|' => "|",
        _ => "",
    }
}

fn command_symbol(command: &str) -> Option<&'static str> {
    Some(match command {
        "alpha" => "\u{03b1}",
        "beta" => "\u{03b2}",
        "gamma" => "\u{03b3}",
        "delta" => "\u{03b4}",
        "epsilon" | "varepsilon" => "\u{03b5}",
        "zeta" => "\u{03b6}",
        "eta" => "\u{03b7}",
        "theta" | "vartheta" => "\u{03b8}",
        "iota" => "\u{03b9}",
        "kappa" => "\u{03ba}",
        "lambda" => "\u{03bb}",
        "mu" => "\u{03bc}",
        "nu" => "\u{03bd}",
        "xi" => "\u{03be}",
        "pi" | "varpi" => "\u{03c0}",
        "rho" | "varrho" => "\u{03c1}",
        "sigma" | "varsigma" => "\u{03c3}",
        "tau" => "\u{03c4}",
        "upsilon" => "\u{03c5}",
        "phi" | "varphi" => "\u{03c6}",
        "chi" => "\u{03c7}",
        "psi" => "\u{03c8}",
        "omega" => "\u{03c9}",
        "Gamma" => "\u{0393}",
        "Delta" => "\u{0394}",
        "Theta" => "\u{0398}",
        "Lambda" => "\u{039b}",
        "Xi" => "\u{039e}",
        "Pi" => "\u{03a0}",
        "Sigma" => "\u{03a3}",
        "Upsilon" => "\u{03a5}",
        "Phi" => "\u{03a6}",
        "Psi" => "\u{03a8}",
        "Omega" => "\u{03a9}",
        "pm" => "\u{00b1}",
        "mp" => "\u{2213}",
        "times" => "\u{00d7}",
        "div" => "\u{00f7}",
        "cdot" | "bullet" => "\u{22c5}",
        "le" | "leq" => "\u{2264}",
        "ge" | "geq" => "\u{2265}",
        "neq" | "ne" => "\u{2260}",
        "approx" => "\u{2248}",
        "equiv" => "\u{2261}",
        "propto" => "\u{221d}",
        "infty" => "\u{221e}",
        "partial" => "\u{2202}",
        "nabla" => "\u{2207}",
        "sum" => "\u{2211}",
        "prod" => "\u{220f}",
        "int" => "\u{222b}",
        "in" => "\u{2208}",
        "notin" => "\u{2209}",
        "subset" => "\u{2282}",
        "subseteq" => "\u{2286}",
        "cup" => "\u{222a}",
        "cap" => "\u{2229}",
        "forall" => "\u{2200}",
        "exists" => "\u{2203}",
        "to" | "rightarrow" | "Rightarrow" => "\u{2192}",
        "leftarrow" | "Leftarrow" => "\u{2190}",
        "leftrightarrow" | "Leftrightarrow" => "\u{2194}",
        "sin" => "sin",
        "cos" => "cos",
        "tan" => "tan",
        "log" => "log",
        "ln" => "ln",
        "lim" => "lim",
        "max" => "max",
        "min" => "min",
        "{" => "{",
        "}" => "}",
        "_" => "_",
        "%" => "%",
        "$" => "$",
        "&" => "&",
        "#" => "#",
        _ => return None,
    })
}

fn svg_color(color: Hsla) -> (String, f32) {
    let rgba: Rgba = color.into();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    (
        format!(
            "#{:02x}{:02x}{:02x}",
            channel(rgba.r),
            channel(rgba.g),
            channel(rgba.b)
        ),
        rgba.a.clamp(0.0, 1.0),
    )
}

fn escape_xml(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use gpui::Hsla;

    use super::*;

    #[test]
    fn renders_fraction_as_svg_with_rule() {
        let svg = render_math_svg(r"\frac{a}{b}", MathDisplay::Block, 16.0, Hsla::black());

        assert!(svg.contains("<svg"));
        assert!(svg.contains("<line"));
        assert!(svg.contains(">a</text>"));
        assert!(svg.contains(">b</text>"));
    }

    #[test]
    fn renders_scripts_and_symbols_as_svg_text() {
        let svg = render_math_svg(
            r"e^{i\pi} + 1 = 0",
            MathDisplay::Inline,
            14.0,
            Hsla::black(),
        );

        assert!(svg.contains("<svg"));
        assert!(svg.contains("&#") || svg.contains("\u{03c0}"));
        assert!(svg.contains(">e</text>"));
        assert!(svg.contains(">i"));
    }

    #[test]
    fn does_not_stroke_text_runs() {
        let svg = render_math_svg("a + b", MathDisplay::Block, 16.0, Hsla::black());

        assert!(
            !svg.contains("<g fill=\"#000000\" fill-opacity=\"1.000\" stroke="),
            "text should not inherit stroke from the group"
        );
    }
}
