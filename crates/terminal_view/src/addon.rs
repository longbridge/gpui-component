//! Terminal Addon Plugin System
//!
//! Similar to xterm.js addon architecture, provides extensible plugin mechanism.

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Point as AlacPoint};
use alacritty_terminal::term::search::RegexSearch;
use alacritty_terminal::term::Term;
use gpui::*;
use std::any::Any;
use std::collections::HashMap;
use std::ops::{Range, RangeInclusive};
use std::path::{Path, PathBuf};
use url::Url;
use rust_i18n::t;

use terminal::pty_backend::GpuiEventProxy;

// ============================================================================
// Decoration System
// ============================================================================

/// Cell decoration type - defines how to decorate terminal cells
#[derive(Clone, Debug)]
pub enum CellDecoration {
    /// Background color decoration
    Background {
        color: Hsla,
        priority: u8, // 0-255, higher number = higher priority
    },

    /// Foreground color decoration
    Foreground { color: Hsla, priority: u8 },

    /// Underline decoration
    Underline {
        color: Hsla,
        thickness: Pixels,
        priority: u8,
    },

    /// Combined decoration (foreground + background)
    Highlight {
        foreground: Hsla,
        background: Hsla,
        priority: u8,
    },
}

impl CellDecoration {
    pub fn priority(&self) -> u8 {
        match self {
            Self::Background { priority, .. } => *priority,
            Self::Foreground { priority, .. } => *priority,
            Self::Underline { priority, .. } => *priority,
            Self::Highlight { priority, .. } => *priority,
        }
    }
}

/// Decoration span - defines where a decoration applies
#[derive(Clone, Debug)]
pub struct DecorationSpan {
    pub line: usize,             // Screen line number
    pub col_range: Range<usize>, // Column range
    pub decoration: CellDecoration,
}

#[derive(Clone, Debug)]
pub struct TerminalAddonTooltip {
    pub action_hint: &'static str,
    pub action_text: &'static str,
    pub display_text: String,
    pub display_color: Hsla,
}

pub struct TerminalAddonMouseContext<'a> {
    pub screen_line: usize,
    pub column: usize,
    pub line_text: &'a str,
    pub modifiers: Modifiers,
    pub position: Point<Pixels>,
    pub is_local: bool,
    pub base_dir: Option<&'a Path>,
    open_url: &'a mut dyn FnMut(&str),
}

impl<'a> TerminalAddonMouseContext<'a> {
    pub fn new(
        screen_line: usize,
        column: usize,
        line_text: &'a str,
        modifiers: Modifiers,
        position: Point<Pixels>,
        is_local: bool,
        base_dir: Option<&'a Path>,
        open_url: &'a mut dyn FnMut(&str),
    ) -> Self {
        Self {
            screen_line,
            column,
            line_text,
            modifiers,
            position,
            is_local,
            base_dir,
            open_url,
        }
    }

    pub fn open_url(&mut self, url: &str) {
        (self.open_url)(url);
    }
}

#[derive(Default, Clone, Copy, Debug)]
pub struct HoverUpdate {
    pub changed: bool,
    pub hovered: bool,
    pub exclusive: bool,
}

pub struct TerminalAddonFrameContext<'a> {
    pub term: &'a Term<GpuiEventProxy>,
    pub visible_lines: Range<usize>,
    pub display_offset: usize,
    pub is_local: bool,
    pub base_dir: Option<&'a Path>,
}

// ============================================================================
// Core Traits
// ============================================================================

/// Terminal addon trait - similar to xterm.js ITerminalAddon
pub trait TerminalAddon: Send + Sync {
    /// Unique identifier for this addon
    fn id(&self) -> &'static str;

    /// Called when the addon is loaded into the terminal
    fn activate(&mut self) {}

    /// Called when the addon is being unloaded
    fn dispose(&mut self) {}

    /// Handle keyboard input before terminal processes it
    /// Return true to consume the event (prevent terminal from handling it)
    fn on_key(&mut self, _event: &KeyDownEvent) -> bool {
        false
    }

    /// Handle terminal resize
    fn on_resize(&mut self, _cols: usize, _rows: usize) {}

    /// Handle scroll events
    fn on_scroll(&mut self, _delta: i32) {}

    /// Handle mouse move events (return whether hover state changed)
    fn on_mouse_move(&mut self, _context: &mut TerminalAddonMouseContext) -> HoverUpdate {
        HoverUpdate::default()
    }

    /// Handle mouse down events (return true to consume)
    fn on_mouse_down(&mut self, _context: &mut TerminalAddonMouseContext) -> bool {
        false
    }

    /// Handle mouse up events (return true to consume)
    fn on_mouse_up(&mut self, _context: &mut TerminalAddonMouseContext) -> bool {
        false
    }

    /// Prepare addon state before rendering
    fn on_frame(&mut self, _context: &TerminalAddonFrameContext) {}

    /// Clear hover state (return true if state changed)
    fn clear_hover(&mut self) -> bool {
        false
    }

    /// Tooltip info for current hover
    fn tooltip(&self) -> Option<TerminalAddonTooltip> {
        None
    }

    /// Provide decorations for visible terminal cells
    ///
    /// # Parameters
    /// - `visible_lines`: Range of visible screen lines
    /// - `display_offset`: Current display offset
    ///
    /// # Returns
    /// Vector of decoration spans that this addon wants to apply
    fn provide_decorations(
        &self,
        _visible_lines: Range<usize>,
        _display_offset: usize,
    ) -> Vec<DecorationSpan> {
        Vec::new() // Default: no decorations
    }

    /// Downcast to concrete type
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

// ============================================================================
// Addon Manager
// ============================================================================

/// Manages loaded addons for a terminal instance
pub struct AddonManager {
    addons: HashMap<&'static str, Box<dyn TerminalAddon>>,
    load_order: Vec<&'static str>,
}

impl Default for AddonManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AddonManager {
    pub fn new() -> Self {
        Self {
            addons: HashMap::new(),
            load_order: Vec::new(),
        }
    }

    /// Load an addon
    pub fn load(&mut self, mut addon: Box<dyn TerminalAddon>) {
        let id = addon.id();
        if self.addons.contains_key(id) {
            return;
        }

        addon.activate();
        self.load_order.push(id);
        self.addons.insert(id, addon);
    }

    /// Unload an addon by id
    pub fn unload(&mut self, id: &str) -> Option<Box<dyn TerminalAddon>> {
        if let Some(mut addon) = self.addons.remove(id) {
            addon.dispose();
            self.load_order.retain(|&x| x != id);
            Some(addon)
        } else {
            None
        }
    }

    /// Get addon by id
    pub fn get(&self, id: &str) -> Option<&dyn TerminalAddon> {
        self.addons.get(id).map(|a| &**a)
    }

    /// Get addon as concrete type
    pub fn get_as<T: 'static>(&self, id: &str) -> Option<&T> {
        self.addons
            .get(id)
            .and_then(|a| a.as_any().downcast_ref::<T>())
    }

    /// Get addon as concrete type (mutable)
    pub fn get_as_mut<T: 'static>(&mut self, id: &str) -> Option<&mut T> {
        self.addons
            .get_mut(id)
            .and_then(|a| a.as_any_mut().downcast_mut::<T>())
    }

    /// Check if addon is loaded
    pub fn is_loaded(&self, id: &str) -> bool {
        self.addons.contains_key(id)
    }

    /// Iterate over all loaded addons
    pub fn iter_addons(&self) -> impl Iterator<Item = &dyn TerminalAddon> {
        self.load_order.iter().filter_map(|id| {
            self.addons
                .get(id)
                .map(|addon| &**addon as &dyn TerminalAddon)
        })
    }

    /// Dispatch key event to all addons
    /// Returns true if any addon consumed the event
    pub fn dispatch_key(&mut self, event: &KeyDownEvent) -> bool {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                if addon.on_key(event) {
                    return true;
                }
            }
        }
        false
    }

    /// Dispatch resize event to all addons
    pub fn dispatch_resize(&mut self, cols: usize, rows: usize) {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                addon.on_resize(cols, rows);
            }
        }
    }

    /// Dispatch scroll event to all addons
    pub fn dispatch_scroll(&mut self, delta: i32) {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                addon.on_scroll(delta);
            }
        }
    }

    /// Dispatch mouse move event to all addons
    pub fn dispatch_mouse_move(&mut self, context: &mut TerminalAddonMouseContext) -> bool {
        let mut changed = false;
        let mut exclusive_id: Option<&'static str> = None;

        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                let update = addon.on_mouse_move(context);
                changed |= update.changed;
                if update.hovered && update.exclusive && exclusive_id.is_none() {
                    exclusive_id = Some(*id);
                }
            }
        }

        if let Some(exclusive_id) = exclusive_id {
            for id in &self.load_order {
                if *id == exclusive_id {
                    continue;
                }
                if let Some(addon) = self.addons.get_mut(id) {
                    changed |= addon.clear_hover();
                }
            }
        }

        changed
    }

    /// Dispatch mouse down event to all addons
    pub fn dispatch_mouse_down(&mut self, context: &mut TerminalAddonMouseContext) -> bool {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                if addon.on_mouse_down(context) {
                    return true;
                }
            }
        }
        false
    }

    /// Dispatch mouse up event to all addons
    pub fn dispatch_mouse_up(&mut self, context: &mut TerminalAddonMouseContext) -> bool {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                if addon.on_mouse_up(context) {
                    return true;
                }
            }
        }
        false
    }

    /// Prepare addons before rendering
    pub fn dispatch_frame(&mut self, context: &TerminalAddonFrameContext) {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get_mut(id) {
                addon.on_frame(context);
            }
        }
    }

    /// Get tooltip from addons
    pub fn tooltip(&self) -> Option<TerminalAddonTooltip> {
        for id in &self.load_order {
            if let Some(addon) = self.addons.get(id) {
                if let Some(tooltip) = addon.tooltip() {
                    return Some(tooltip);
                }
            }
        }
        None
    }
}

impl Drop for AddonManager {
    fn drop(&mut self) {
        for id in self.load_order.drain(..).collect::<Vec<_>>() {
            if let Some(mut addon) = self.addons.remove(id) {
                addon.dispose();
            }
        }
    }
}

// ============================================================================
// Built-in Addons
// ============================================================================

/// WebLinks Addon - Detect and handle URLs
pub struct WebLinksAddon {
    url_regex: regex::Regex,
    hovered_link: Option<HoveredLink>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoveredLink {
    pub url: String,
    pub line: usize,
    pub col_range: Range<usize>,
}

impl WebLinksAddon {
    pub fn new() -> Self {
        Self {
            url_regex: regex::Regex::new(
                r"(?i)(https?://|file://|mailto:|git://|ssh://)[^\s<>\[\]{}|\\^`\x00-\x1f\x7f]+",
            )
            .expect("Invalid URL regex"),
            hovered_link: None,
        }
    }

    /// Get currently hovered link
    pub fn hovered_link(&self) -> Option<&HoveredLink> {
        self.hovered_link.as_ref()
    }

    /// Clear hovered link
    pub fn clear_hovered(&mut self) {
        self.hovered_link = None;
    }

    /// Detect URL at position in line text
    pub fn detect_url_at(&mut self, line_text: &str, col: usize, screen_line: usize) -> bool {
        self.hovered_link = None;

        if line_text.is_empty() {
            return false;
        }

        for mat in self.url_regex.find_iter(line_text) {
            let start_col = line_text[..mat.start()].chars().count();
            let end_col = line_text[..mat.end()].chars().count();

            if col >= start_col && col < end_col {
                self.hovered_link = Some(HoveredLink {
                    url: mat.as_str().to_string(),
                    line: screen_line,
                    col_range: start_col..end_col,
                });
                return true;
            }
        }
        false
    }
}

impl Default for WebLinksAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalAddon for WebLinksAddon {
    fn id(&self) -> &'static str {
        "weblinks"
    }

    fn on_mouse_move(&mut self, context: &mut TerminalAddonMouseContext) -> HoverUpdate {
        let previous = self.hovered_link.clone();
        let matched = self.detect_url_at(context.line_text, context.column, context.screen_line);
        let changed = previous != self.hovered_link;

        HoverUpdate {
            changed,
            hovered: matched,
            exclusive: matched,
        }
    }

    fn on_mouse_down(&mut self, context: &mut TerminalAddonMouseContext) -> bool {
        if !context.modifiers.platform {
            return false;
        }

        let matched = self.detect_url_at(context.line_text, context.column, context.screen_line);
        if !matched {
            return false;
        }

        if let Some(link) = self.hovered_link.as_ref() {
            context.open_url(&link.url);
            return true;
        }

        false
    }

    fn clear_hover(&mut self) -> bool {
        if self.hovered_link.is_some() {
            self.hovered_link = None;
            return true;
        }
        false
    }

    fn tooltip(&self) -> Option<TerminalAddonTooltip> {
        self.hovered_link.as_ref().map(|link| TerminalAddonTooltip {
            action_hint: "⌘ + Click",
            action_text: "to open the link",
            display_text: link.url.clone(),
            display_color: rgb(0x66ccff).into(),
        })
    }

    fn provide_decorations(
        &self,
        _visible_lines: Range<usize>,
        _display_offset: usize,
    ) -> Vec<DecorationSpan> {
        if let Some(ref link) = self.hovered_link {
            vec![DecorationSpan {
                line: link.line,
                col_range: link.col_range.clone(),
                decoration: CellDecoration::Foreground {
                    color: hsla(0.55, 0.8, 0.6, 1.0), // Cyan color for links
                    priority: 50,                     // Medium priority
                },
            }]
        } else {
            Vec::new()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Search Addon - Text search functionality
pub struct SearchAddon {
    regex: Option<RegexSearch>,
    current_match: Option<RangeInclusive<AlacPoint>>,
    pattern: String,
}

impl SearchAddon {
    pub fn new() -> Self {
        Self {
            regex: None,
            current_match: None,
            pattern: String::new(),
        }
    }

    /// Set search pattern
    pub fn set_pattern(&mut self, pattern: &str) -> Result<(), regex::Error> {
        if pattern.is_empty() {
            self.regex = None;
            self.current_match = None;
            self.pattern.clear();
            return Ok(());
        }

        let regex = RegexSearch::new(pattern).map_err(|e| regex::Error::Syntax(e.to_string()))?;
        self.regex = Some(regex);
        self.current_match = None;
        self.pattern = pattern.to_string();
        Ok(())
    }

    /// Get current pattern
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Get current match
    pub fn current_match(&self) -> Option<&RangeInclusive<AlacPoint>> {
        self.current_match.as_ref()
    }

    /// Clear search
    pub fn clear(&mut self) {
        self.regex = None;
        self.current_match = None;
        self.pattern.clear();
    }

    /// Find last match in terminal (search from bottom to top)
    pub fn find_last(
        &mut self,
        term: &mut Term<GpuiEventProxy>,
    ) -> Option<RangeInclusive<AlacPoint>> {
        use alacritty_terminal::index::{Column, Direction, Side};

        let regex = self.regex.as_mut()?;
        let bottom = AlacPoint::new(term.bottommost_line(), Column(term.columns() - 1));

        if let Some(match_) = term.search_next(regex, bottom, Direction::Left, Side::Right, None) {
            term.scroll_to_point(*match_.start());
            self.current_match = Some(match_.clone());
            Some(match_)
        } else {
            None
        }
    }

    /// Find next match in terminal (with wrap around)
    pub fn find_next(
        &mut self,
        term: &mut Term<GpuiEventProxy>,
    ) -> Option<RangeInclusive<AlacPoint>> {
        use alacritty_terminal::index::{Column, Direction, Side};

        let regex = self.regex.as_mut()?;
        let origin = if let Some(ref current) = self.current_match {
            *current.end()
        } else {
            term.grid().cursor.point
        };

        let result = term.search_next(regex, origin, Direction::Right, Side::Left, None);

        let match_ = if result.is_none() {
            let top = AlacPoint::new(term.topmost_line(), Column(0));
            term.search_next(regex, top, Direction::Right, Side::Left, None)
        } else {
            result
        };

        if let Some(match_) = match_ {
            term.scroll_to_point(*match_.start());
            self.current_match = Some(match_.clone());
            Some(match_)
        } else {
            None
        }
    }

    /// Find previous match in terminal (with wrap around)
    pub fn find_previous(
        &mut self,
        term: &mut Term<GpuiEventProxy>,
    ) -> Option<RangeInclusive<AlacPoint>> {
        use alacritty_terminal::index::{Column, Direction, Side};

        let regex = self.regex.as_mut()?;
        let origin = if let Some(ref current) = self.current_match {
            *current.start()
        } else {
            term.grid().cursor.point
        };

        let result = term.search_next(regex, origin, Direction::Left, Side::Right, None);

        let match_ = if result.is_none() {
            let bottom = AlacPoint::new(term.bottommost_line(), Column(term.columns() - 1));
            term.search_next(regex, bottom, Direction::Left, Side::Right, None)
        } else {
            result
        };

        if let Some(match_) = match_ {
            term.scroll_to_point(*match_.start());
            self.current_match = Some(match_.clone());
            Some(match_)
        } else {
            None
        }
    }

    /// Check if search is active
    pub fn is_active(&self) -> bool {
        self.regex.is_some()
    }
}

impl Default for SearchAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalAddon for SearchAddon {
    fn id(&self) -> &'static str {
        "search"
    }

    fn provide_decorations(
        &self,
        _visible_lines: Range<usize>,
        display_offset: usize,
    ) -> Vec<DecorationSpan> {
        if let Some(ref match_range) = self.current_match {
            let start = match_range.start();
            let end = match_range.end();

            // Convert AlacPoint to screen coordinates
            let start_line = (start.line.0 + display_offset as i32) as usize;
            let end_line = (end.line.0 + display_offset as i32) as usize;

            if start_line == end_line {
                // Single line match
                vec![DecorationSpan {
                    line: start_line,
                    col_range: start.column.0..end.column.0 + 1,
                    decoration: CellDecoration::Highlight {
                        foreground: hsla(0.0, 0.0, 0.0, 1.0),  // Black text
                        background: hsla(0.15, 0.8, 0.5, 1.0), // Yellow highlight
                        priority: 100,                         // High priority
                    },
                }]
            } else {
                // Multi-line match - not common but handle it
                Vec::new()
            }
        } else {
            Vec::new()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// IP Address Highlight Addon - Highlight IP addresses in terminal
pub struct IpHighlightAddon {
    ipv4_regex: regex::Regex,
    ipv6_regex: regex::Regex,
    highlight_color: (u8, u8, u8),
    enabled: bool,
    cached_ips: Vec<IpAddress>, // Cached IP addresses for current frame
}

#[derive(Clone, Debug)]
pub struct IpAddress {
    pub ip: String,
    pub line: usize,
    pub col_range: Range<usize>,
}

impl IpHighlightAddon {
    pub fn new() -> Self {
        Self {
            ipv4_regex: regex::Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b",
            )
            .expect("Invalid IPv4 regex"),
            ipv6_regex: regex::Regex::new(
                r"\b(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}\b|\b(?:[0-9a-fA-F]{1,4}:){1,7}:\b|\b(?:[0-9a-fA-F]{1,4}:){1,6}:[0-9a-fA-F]{1,4}\b",
            )
            .expect("Invalid IPv6 regex"),
            highlight_color: (100, 200, 255),
            enabled: true,
            cached_ips: Vec::new(),
        }
    }

    /// Update cached IP addresses (call this before rendering)
    pub fn update_cache(&mut self, ips: Vec<IpAddress>) {
        self.cached_ips = ips;
    }

    /// Set highlight color (RGB)
    pub fn set_color(&mut self, r: u8, g: u8, b: u8) {
        self.highlight_color = (r, g, b);
    }

    /// Get current highlight color
    pub fn color(&self) -> (u8, u8, u8) {
        self.highlight_color
    }

    /// Enable or disable IP highlighting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if highlighting is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Detect all IP addresses in a line
    pub fn detect_ips_in_line(&self, line_text: &str, screen_line: usize) -> Vec<IpAddress> {
        if !self.enabled || line_text.is_empty() {
            return Vec::new();
        }

        let mut ips = Vec::new();

        // Detect IPv4 addresses
        for mat in self.ipv4_regex.find_iter(line_text) {
            let start_col = line_text[..mat.start()].chars().count();
            let end_col = line_text[..mat.end()].chars().count();

            ips.push(IpAddress {
                ip: mat.as_str().to_string(),
                line: screen_line,
                col_range: start_col..end_col,
            });
        }

        // Detect IPv6 addresses
        for mat in self.ipv6_regex.find_iter(line_text) {
            let start_col = line_text[..mat.start()].chars().count();
            let end_col = line_text[..mat.end()].chars().count();

            ips.push(IpAddress {
                ip: mat.as_str().to_string(),
                line: screen_line,
                col_range: start_col..end_col,
            });
        }

        ips
    }
}

impl Default for IpHighlightAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalAddon for IpHighlightAddon {
    fn id(&self) -> &'static str {
        "ip_highlight"
    }

    fn on_frame(&mut self, context: &TerminalAddonFrameContext) {
        if !self.enabled {
            self.cached_ips.clear();
            return;
        }

        let term = context.term;
        let content = term.renderable_content();
        let display_offset = content.display_offset;

        let mut ips = Vec::new();
        let mut seen_lines = std::collections::HashSet::new();

        for cell in content.display_iter {
            let screen_line = cell.point.line.0 + display_offset as i32;
            if screen_line < 0 {
                continue;
            }
            let line_idx = screen_line as usize;

            if !context.visible_lines.contains(&line_idx) || seen_lines.contains(&line_idx) {
                continue;
            }

            seen_lines.insert(line_idx);

            let grid = term.grid();
            let mut line_text = String::new();
            for col in 0..term.columns() {
                let cell = &grid[cell.point.line][Column(col)];
                if cell.c != '\0' {
                    line_text.push(cell.c);
                }
            }

            ips.extend(self.detect_ips_in_line(&line_text, line_idx));
        }

        self.update_cache(ips);
    }

    fn provide_decorations(
        &self,
        visible_lines: Range<usize>,
        _display_offset: usize,
    ) -> Vec<DecorationSpan> {
        if !self.enabled {
            return Vec::new();
        }

        let (r, g, b) = self.highlight_color;
        let color = Rgba {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: 1.0,
        }
        .into();

        self.cached_ips
            .iter()
            .filter(|ip| visible_lines.contains(&ip.line))
            .map(|ip| DecorationSpan {
                line: ip.line,
                col_range: ip.col_range.clone(),
                decoration: CellDecoration::Foreground {
                    color,
                    priority: 40, // Lower priority than search
                },
            })
            .collect()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// File Path Addon - Detect and handle local file paths
pub struct FilePathAddon {
    path_regex: Option<regex::Regex>,
    hovered_path: Option<HoveredPath>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HoveredPath {
    pub display: String,
    pub path: PathBuf,
    pub line: usize,
    pub col_range: Range<usize>,
}

impl FilePathAddon {
    pub fn new() -> Self {
        let path_regex = regex::Regex::new(
            r#"(?x)
            (?P<path>
                (?:~|\.{1,2})/[^\s:<>"'|]+|
                /[^\s:<>"'|]+|
                [A-Za-z]:\\[^\s:<>"'|]+|
                \\\\[^\s:<>"'|]+
            )
            (?::\d+)?(?::\d+)?
            "#,
        )
        .map_err(|error| {
            tracing::warn!(
                "{}",
                t!("TerminalAddon.path_regex_build_failed", error = error).to_string()
            );
            error
        })
        .ok();

        Self {
            path_regex,
            hovered_path: None,
        }
    }

    pub fn hovered_path(&self) -> Option<&HoveredPath> {
        self.hovered_path.as_ref()
    }

    pub fn clear_hovered(&mut self) {
        self.hovered_path = None;
    }

    pub fn detect_path_at(
        &mut self,
        line_text: &str,
        column: usize,
        screen_line: usize,
        base_dir: Option<&Path>,
    ) -> bool {
        self.hovered_path = None;

        if line_text.is_empty() {
            return false;
        }

        let path_regex = match self.path_regex.as_ref() {
            Some(regex) => regex,
            None => return false,
        };

        for mat in path_regex.find_iter(line_text) {
            let start_col = line_text[..mat.start()].chars().count();
            let end_col = line_text[..mat.end()].chars().count();

            if column < start_col || column >= end_col {
                continue;
            }

            let candidate = mat.as_str();
            if candidate.starts_with("file://") {
                continue;
            }

            let (path_part, _line_number, _column_number) = split_path_line_column(candidate);
            let cleaned_path = trim_trailing_punctuation(&path_part);

            if let Some(resolved_path) = resolve_path(&cleaned_path, base_dir) {
                if resolved_path.exists() {
                    self.hovered_path = Some(HoveredPath {
                        display: candidate.to_string(),
                        path: resolved_path,
                        line: screen_line,
                        col_range: start_col..end_col,
                    });
                    return true;
                }
            }
        }

        false
    }
}

impl Default for FilePathAddon {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalAddon for FilePathAddon {
    fn id(&self) -> &'static str {
        "file_paths"
    }

    fn on_mouse_move(&mut self, context: &mut TerminalAddonMouseContext) -> HoverUpdate {
        if !context.is_local {
            let changed = self.clear_hover();
            return HoverUpdate {
                changed,
                hovered: false,
                exclusive: false,
            };
        }

        let previous = self.hovered_path.clone();
        let matched = self.detect_path_at(
            context.line_text,
            context.column,
            context.screen_line,
            context.base_dir,
        );
        let changed = previous != self.hovered_path;

        HoverUpdate {
            changed,
            hovered: matched,
            exclusive: matched,
        }
    }

    fn on_mouse_down(&mut self, context: &mut TerminalAddonMouseContext) -> bool {
        if !context.is_local || !context.modifiers.platform {
            return false;
        }

        let matched = self.detect_path_at(
            context.line_text,
            context.column,
            context.screen_line,
            context.base_dir,
        );

        if !matched {
            return false;
        }

        if let Some(path) = self.hovered_path.as_ref() {
            if let Some(url) = file_path_to_url(&path.path) {
                context.open_url(&url);
                return true;
            }
        }

        false
    }

    fn clear_hover(&mut self) -> bool {
        if self.hovered_path.is_some() {
            self.hovered_path = None;
            return true;
        }
        false
    }

    fn tooltip(&self) -> Option<TerminalAddonTooltip> {
        self.hovered_path.as_ref().map(|path| TerminalAddonTooltip {
            action_hint: "⌘ + Click",
            action_text: "to open the path",
            display_text: path.display.clone(),
            display_color: rgb(0x9be58e).into(),
        })
    }

    fn provide_decorations(
        &self,
        _visible_lines: Range<usize>,
        _display_offset: usize,
    ) -> Vec<DecorationSpan> {
        if let Some(ref hovered) = self.hovered_path {
            vec![DecorationSpan {
                line: hovered.line,
                col_range: hovered.col_range.clone(),
                decoration: CellDecoration::Foreground {
                    color: hsla(0.33, 0.75, 0.55, 1.0),
                    priority: 45,
                },
            }]
        } else {
            Vec::new()
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub fn register_default_addons(manager: &mut AddonManager) {
    manager.load(Box::new(WebLinksAddon::new()));
    manager.load(Box::new(FilePathAddon::new()));
    manager.load(Box::new(SearchAddon::new()));
    manager.load(Box::new(IpHighlightAddon::new()));
}

fn split_path_line_column(candidate: &str) -> (String, Option<u32>, Option<u32>) {
    let mut path = candidate.to_string();
    let mut column_number = None;
    let mut line_number = None;

    if let Some((base, column)) = split_trailing_number(&path) {
        column_number = Some(column);
        path = base.to_string();
    }

    if let Some((base, line)) = split_trailing_number(&path) {
        line_number = Some(line);
        path = base.to_string();
    }

    (path, line_number, column_number)
}

fn split_trailing_number(candidate: &str) -> Option<(&str, u32)> {
    let (base, suffix) = candidate.rsplit_once(':')?;
    if suffix.is_empty() || !suffix.chars().all(|char| char.is_ascii_digit()) {
        return None;
    }
    let number = suffix.parse().ok()?;
    Some((base, number))
}

fn trim_trailing_punctuation(candidate: &str) -> String {
    let trimmed = candidate.trim_end_matches(|char: char| matches!(char, ')' | ']' | '}' | ',' | ';'));
    trimmed.to_string()
}

fn resolve_path(raw_path: &str, base_dir: Option<&Path>) -> Option<PathBuf> {
    if raw_path.is_empty() {
        return None;
    }

    let expanded = if raw_path == "~" {
        expand_home("")?
    } else if let Some(stripped) = raw_path.strip_prefix("~/") {
        expand_home(stripped)?
    } else {
        PathBuf::from(raw_path)
    };

    if expanded.is_absolute() {
        return Some(expanded);
    }

    if let Some(base) = base_dir {
        return Some(base.join(expanded));
    }

    let current_dir = std::env::current_dir().ok()?;
    Some(current_dir.join(expanded))
}

fn expand_home(suffix: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;

    if suffix.is_empty() {
        return Some(home);
    }

    Some(home.join(suffix))
}

fn file_path_to_url(path: &Path) -> Option<String> {
    let resolved = match path.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            tracing::warn!(
                "{}",
                t!("TerminalAddon.open_local_path_failed", error = error).to_string()
            );
            return None;
        }
    };

    let url = match Url::from_file_path(&resolved) {
        Ok(url) => url.to_string(),
        Err(()) => format!("file://{}", resolved.display()),
    };

    Some(url)
}
