#!/bin/bash

# Merge process:
#
# * Use mergiraf for merge, with `git merge main -X theirs`
#
#    - Need to use it with a patched tree-sitter-rust. I (Michael)
#      haven't yet uploaded a fork for this, can do if helpful.
#      https://github.com/tree-sitter/tree-sitter-rust/pull/245
#
#    - Watch for newlines between top level decls sometimes disappearing
#
# * Run this script.

# Check if `ruplacer` is installed
if ! command -v ruplacer &> /dev/null
then
    echo "Please install it with `cargo install ruplacer`"
    exit
fi

dry=true
if [ "$1" = "apply" ]; then
    dry=false
fi

re() {
    echo "$1" "    -->    " "$2"
    if [ "$dry" = true ]; then
        ruplacer "$1" "$2" crates/ --type *.rs
    else
        ruplacer "$1" "$2" crates/ --type *.rs --go
    fi
}

re '\.new_view\('                    '.new_model('
re 'cx.view\('                       'cx.model('
re '\.observe_new_views\('           '.observe_new_models('
re ': AppContext'                     ': App'
re 'View<'                           'Entity<'
re 'WeakView<'                        'WeakEntity<'
re 'Model<'                          'Entity<'
re 'WeakModel<'                       'WeakEntity<'
re 'FocusableView'                   'Focusable'

# closure parameters
re '&AppContext'          '&App'
re '&gpui::AppContext'          '&gpui::App'
re '&mut gpui::AppContext'          '&mut gpui::App'
re ', &mut WindowContext'          ', &mut Window, &mut App'
re ', &mut gpui::WindowContext'          ', &mut gpui::Window, &mut gpui::App'
re ', &mut WindowContext'          ', &mut Window, &mut App'
re ', &mut gpui::WindowContext'          ', &mut gpui::Window, &mut gpui::App'
re ', &mut ViewContext<([^>]+)>'   ', &mut Window, &mut Context<$1>'
re ', &mut gpui::ViewContext<([^>]+)>'   ', &mut gpui::Window, &mut gpui::Context<$1>'
re '\(&mut WindowContext'          '(&mut Window, &mut App'
re '\(&mut gpui::WindowContext'          '(&mut gpui::Window, &mut gpui::App'
re '\(&mut ViewContext<([^>]+)>'   '(&mut Window, &mut Context<$1>'
re '\(&mut gpui::ViewContext<([^>]+)>'   '(&mut gpui::Window, &mut gpui::Context<$1>'

# function parameters
re '_: &mut WindowContext\)'          '_window: &mut Window, _cx: &mut App)'
re '_: &mut gpui::WindowContext\)'          '_window: &mut gpui::Window, _cx: &mut gpui::App)'
re '_: &mut ViewContext<([^>]+)>\)'   '_window: &mut Window, _cx: &mut Context<$1>)'
re '_: &mut gpui::ViewContext<([^>]+)>\)'   '_window: &mut gpui::Window, _cx: &mut gpui::Context<$1>)'
re '_: &mut WindowContext,'           '_window: &mut Window, _cx: &mut App,'
re '_: &mut gpui::WindowContext,'           '_window: &mut gpui::Window, _cx: &mut gpui::App,'
re '_: &mut ViewContext<([^>]+)>,'    '_window: &mut Window, _cx: &mut Context<$1>,'
re '_: &mut gpui::ViewContext<([^>]+)>,'    '_window: &mut gpui::Window, _cx: &mut gpui::Context<$1>,'
re '_cx: &mut WindowContext\)'        '_window: &mut Window, _cx: &mut App)'
re '_cx: &mut gpui::WindowContext\)'        '_window: &mut gpui::Window, _cx: &mut gpui::App)'
re '_cx: &mut ViewContext<([^>]+)>\)' '_window: &mut Window, _cx: &mut Context<$1>)'
re '_cx: &mut gpui::ViewContext<([^>]+)>\)' '_window: &mut gpui::Window, _cx: &mut gpui::Context<$1>)'
re '_cx: &mut WindowContext,'         '_window: &mut Window, _cx: &mut App,'
re '_cx: &mut gpui::WindowContext,'         '_window: &mut gpui::Window, _cx: &mut gpui::App,'
re '_cx: &mut ViewContext<([^>]+)>,'  '_window: &mut Window, _cx: &mut Context<$1>,'
re '_cx: &mut gpui::ViewContext<([^>]+)>,'  '_window: &mut gpui::Window, _cx: &mut gpui::Context<$1>,'
re 'cx: &mut WindowContext\)'         'window: &mut Window, cx: &mut App)'
re 'cx: &mut gpui::WindowContext\)'         'window: &mut gpui::Window, cx: &mut gpui::App)'
re 'cx: &mut ViewContext<([^>]+)>\)'  'window: &mut Window, cx: &mut Context<$1>)'
re 'cx: &mut gpui::ViewContext<([^>]+)>\)'  'window: &mut gpui::Window, cx: &mut gpui::Context<$1>)'
re 'cx: &mut WindowContext,'          'window: &mut Window, cx: &mut App,'
re 'cx: &mut gpui::WindowContext,'          'window: &mut gpui::Window, cx: &mut gpui::Context,'
re 'cx: &mut ViewContext<([^>]+)>,'   'window: &mut Window, cx: &mut Context<$1>,'
re 'cx: &mut gpui::ViewContext<([^>]+)>,'   'window: &mut gpui::Window, cx: &mut gpui::Context<$1>,'

re '_: &WindowContext\)'              '_window: &Window, _cx: &App)'
re '_: &gpui::WindowContext\)'              '_window: &gpui::Window, _cx: &gpui::App)'
re '_: &ViewContext<([^>]+)>\)'       '_window: &Window, _cx: &Context<$1>)'
re '_: &gpui::ViewContext<([^>]+)>\)'       '_window: &gpui::Window, _cx: &gpui::Context<$1>)'
re '_: &WindowContext,'               '_window: &Window, _cx: &App,'
re '_: &gpui::WindowContext,'               '_window: &gpui::Window, _cx: &gpui::App,'
re '_: &ViewContext<([^>]+)>,'        '_window: &Window, _cx: &Context<$1>,'
re '_: &gpui::ViewContext<([^>]+)>,'        '_window: &gpui::Window, _cx: &gpui::Context<$1>,'
re '_cx: &WindowContext\)'            '_window: &Window, _cx: &App)'
re '_cx: &gpui::WindowContext\)'            '_window: &gpui::Window, _cx: &gpui::App)'
re '_cx: &ViewContext<([^>]+)>\)'     '_window: &Window, _cx: &Context<$1>)'
re '_cx: &gpui::ViewContext<([^>]+)>\)'     '_window: &gpui::Window, _cx: &gpui::Context<$1>)'
re '_cx: &WindowContext,'             '_window: &Window, _cx: &App,'
re '_cx: &gpui::WindowContext,'             '_window: &gpui::Window, _cx: &gpui::App,'
re '_cx: &ViewContext<([^>]+)>,'      '_window: &Window, _cx: &Context<$1>,'
re '_cx: &gpui::ViewContext<([^>]+)>,'      '_window: &gpui::Window, _cx: &gpui::Context<$1>,'
re 'cx: &WindowContext\)'             'window: &Window, cx: &App)'
re 'cx: &gpui::WindowContext\)'             'window: &gpui::Window, cx: &gpui::App)'
re 'cx: &ViewContext<([^>]+)>\)'      'window: &Window, cx: &Context<$1>)'
re 'cx: &gpui::ViewContext<([^>]+)>\)'      'window: &gpui::Window, cx: &gpui::Context<$1>)'
re 'cx: &WindowContext,'              'window: &Window, cx: &App,'
re 'cx: &gpui::WindowContext,'              'window: &gpui::Window, cx: &gpui::App,'
re 'cx: &ViewContext<([^>]+)>,'       'window: &Window, cx: &Context<$1>,'
re 'cx: &gpui::ViewContext<([^>]+)>,'       'window: &gpui::Window, cx: &gpui::Context<$1>,'
re 'cx: &mut WindowContext\|'         'window: &mut Window, cx: &mut App|'

# VisualContext methods moved to window, that take context
re 'cx.dismiss_view\(' 'window.dismiss_view(cx, '
re 'cx.focus_view\(' 'window.focus_view(cx, '
re 'cx.new_view\(' 'cx.new('
re 'cx.new_model\(' 'cx.new('
re 'cx.replace_root_view\(' 'window.replace_root_view(cx, '

# AppContext methods moved to window, that take context
re 'cx.appearance_changed\(\)' 'window.appearance_changed(cx)'
re 'cx.available_actions\(\)' 'window.available_actions(cx)'
re 'cx.dispatch_keystroke_observers\(' 'window.dispatch_keystroke_observers(cx, '
re 'cx.display\(\)' 'window.display(cx)'
re 'cx.focused\(\)' 'window.focused(cx)'
re 'cx.handle_input\(' 'window.handle_input(cx, '
re 'cx.paint_svg\(' 'window.paint_svg(cx, '
re 'cx.request_layout\(' 'window.request_layout(cx, '
re 'cx.use_asset\(' 'window.use_asset(cx, '

# Subset of AppContext methods moved to window that don't take context
re 'cx\.set_cursor_style\('           'window.set_cursor_style('
re 'cx\.modifiers\('                  'window.modifiers('
re 'cx\.mouse_position\('             'window.mouse_position('
re 'cx\.text_style\('                 'window.text_style('
re 'cx\.line_height\('                'window.line_height('

# common closure patterns
re 'cx.listener\(move \|this, _, cx\|' 'cx.listener(move |this, _, window, cx|'
re 'cx.listener\(\|this, _, cx\|'     'cx.listener(|this, _, window, cx|'
re 'cx.listener\(move \|_, _, cx\|'   'cx.listener(move |_, _, window, cx|'
re 'cx.listener\(\|_, _, cx\|'        'cx.listener(|_, _, window, cx|'
re '\.on_click\(move \|_, cx\|'       '.on_click(move |_, window, cx|'
re '\.on_mouse_move\(\|_, cx\|'       '.on_mouse_move(|_, window, cx|'

# cleanup imports
re ' ViewContext,'                     ''
re ' WindowContext,'                   ''
re ' WeakView,'                        ''
re ' View,'                            ''
re ', ViewContext\}'                   '}'
re ', WindowContext\}'                 '}'
re ', WeakView\}'                      '}'
re ', View\}'                          '}'

# other patterns
re '\.detach_and_notify_err\(cx'       '.detach_and_notify_err(window, cx'

re 'cx.bounds\(' 'window.bounds(cx, '
re 'cx.refresh\(' 'window.refresh('
re 'cx.window_bounds\(' 'window.window_bounds('
re 'cx.rem_size\(' 'window.rem_size('
re 'cx.with_content_mask\(' 'window.with_content_mask('
re 'cx.prevent_default\(' 'window.prevent_default('
re 'cx.remove_window\(' 'window.remove_window('
re 'cx.insert_hitbox\(' 'window.insert_hitbox('
re 'cx.window_decorations\(' 'window.window_decorations('
re 'cx.set_client_inset\(' 'window.set_client_inset('
re 'cx.start_window_move\(' 'window.start_window_move('
re 'cx.on_mouse_event\(' 'window.on_mouse_event('
re 'cx.show_window_menu\(' 'window.show_window_menu('
