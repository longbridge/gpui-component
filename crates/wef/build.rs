use std::path::{Path, PathBuf};

use cc::Build;

/// Return the CEF_ROOT env or default path: `$HOME/.cef`
fn cef_root() -> PathBuf {
    if let Ok(path) = std::env::var("CEF_ROOT") {
        return Path::new(&path).to_path_buf();
    }

    dirs::home_dir().expect("get home_dir").join(".cef")
}

fn main() {
    let cef_root = cef_root();
    println!("cargo::rerun-if-changed={}", cef_root.display());

    let profile = match std::env::var("DEBUG") {
        Ok(s) if s != "false" => "Debug",
        _ => "Release",
    };

    let cef_link_search_path = cef_root.join(profile);
    println!(
        "cargo:rustc-link-search=native={}",
        cef_link_search_path.display()
    );

    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=libcef");
    } else if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=cef");
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=AppKit");
        println!("cargo:rustc-link-lib=sandbox");

        // FIXME: Failed to link to `cef_sandbox.a` on macOS/
        //
        // Workaround: copy `cef_sandbox.a` to the output directory and link it as
        // `libcef_sandbox.a`
        //
        // https://github.com/rust-lang/rust/issues/132264
        let outdir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
        std::fs::copy(
            cef_link_search_path.join("cef_sandbox.a"),
            outdir.join("libcef_sandbox.a"),
        )
        .expect("copy cef_sandbox.a");
        println!("cargo:rustc-link-search=native={}", outdir.display());
        println!("cargo:rustc-link-lib=static=cef_sandbox");
    }

    build_dll_wrapper(&cef_root);
    build_wef_sys(&cef_root);
}

fn build_dll_wrapper(cef_root: &Path) {
    #[allow(unused_mut)]
    let mut sources = vec![
        "shutdown_checker.cc",
        "transfer_util.cc",
        "base/cef_atomic_flag.cc",
        "base/cef_callback_helpers.cc",
        "base/cef_callback_internal.cc",
        "base/cef_dump_without_crashing.cc",
        "base/cef_lock.cc",
        "base/cef_lock_impl.cc",
        "base/cef_logging.cc",
        "base/cef_ref_counted.cc",
        "base/cef_thread_checker_impl.cc",
        "base/cef_weak_ptr.cc",
        "cpptoc/accessibility_handler_cpptoc.cc",
        "cpptoc/app_cpptoc.cc",
        "cpptoc/audio_handler_cpptoc.cc",
        "cpptoc/base_ref_counted_cpptoc.cc",
        "cpptoc/base_scoped_cpptoc.cc",
        "cpptoc/browser_process_handler_cpptoc.cc",
        "cpptoc/client_cpptoc.cc",
        "cpptoc/command_handler_cpptoc.cc",
        "cpptoc/completion_callback_cpptoc.cc",
        "cpptoc/context_menu_handler_cpptoc.cc",
        "cpptoc/cookie_access_filter_cpptoc.cc",
        "cpptoc/cookie_visitor_cpptoc.cc",
        "cpptoc/delete_cookies_callback_cpptoc.cc",
        "cpptoc/dev_tools_message_observer_cpptoc.cc",
        "cpptoc/dialog_handler_cpptoc.cc",
        "cpptoc/display_handler_cpptoc.cc",
        "cpptoc/domvisitor_cpptoc.cc",
        "cpptoc/download_handler_cpptoc.cc",
        "cpptoc/download_image_callback_cpptoc.cc",
        "cpptoc/drag_handler_cpptoc.cc",
        "cpptoc/end_tracing_callback_cpptoc.cc",
        "cpptoc/find_handler_cpptoc.cc",
        "cpptoc/focus_handler_cpptoc.cc",
        "cpptoc/frame_handler_cpptoc.cc",
        "cpptoc/jsdialog_handler_cpptoc.cc",
        "cpptoc/keyboard_handler_cpptoc.cc",
        "cpptoc/life_span_handler_cpptoc.cc",
        "cpptoc/load_handler_cpptoc.cc",
        "cpptoc/media_observer_cpptoc.cc",
        "cpptoc/media_route_create_callback_cpptoc.cc",
        "cpptoc/media_sink_device_info_callback_cpptoc.cc",
        "cpptoc/menu_model_delegate_cpptoc.cc",
        "cpptoc/navigation_entry_visitor_cpptoc.cc",
        "cpptoc/pdf_print_callback_cpptoc.cc",
        "cpptoc/permission_handler_cpptoc.cc",
        "cpptoc/preference_observer_cpptoc.cc",
        "cpptoc/print_handler_cpptoc.cc",
        "cpptoc/read_handler_cpptoc.cc",
        "cpptoc/render_handler_cpptoc.cc",
        "cpptoc/render_process_handler_cpptoc.cc",
        "cpptoc/request_context_handler_cpptoc.cc",
        "cpptoc/request_handler_cpptoc.cc",
        "cpptoc/resolve_callback_cpptoc.cc",
        "cpptoc/resource_bundle_handler_cpptoc.cc",
        "cpptoc/resource_handler_cpptoc.cc",
        "cpptoc/resource_request_handler_cpptoc.cc",
        "cpptoc/response_filter_cpptoc.cc",
        "cpptoc/run_file_dialog_callback_cpptoc.cc",
        "cpptoc/scheme_handler_factory_cpptoc.cc",
        "cpptoc/server_handler_cpptoc.cc",
        "cpptoc/set_cookie_callback_cpptoc.cc",
        "cpptoc/setting_observer_cpptoc.cc",
        "cpptoc/string_visitor_cpptoc.cc",
        "cpptoc/task_cpptoc.cc",
        "cpptoc/urlrequest_client_cpptoc.cc",
        "cpptoc/v8_accessor_cpptoc.cc",
        "cpptoc/v8_array_buffer_release_callback_cpptoc.cc",
        "cpptoc/v8_handler_cpptoc.cc",
        "cpptoc/v8_interceptor_cpptoc.cc",
        "cpptoc/write_handler_cpptoc.cc",
        "cpptoc/views/browser_view_delegate_cpptoc.cc",
        "cpptoc/views/button_delegate_cpptoc.cc",
        "cpptoc/views/menu_button_delegate_cpptoc.cc",
        "cpptoc/views/panel_delegate_cpptoc.cc",
        "cpptoc/views/textfield_delegate_cpptoc.cc",
        "cpptoc/views/view_delegate_cpptoc.cc",
        "cpptoc/views/window_delegate_cpptoc.cc",
        "ctocpp/auth_callback_ctocpp.cc",
        "ctocpp/before_download_callback_ctocpp.cc",
        "ctocpp/binary_value_ctocpp.cc",
        "ctocpp/browser_ctocpp.cc",
        "ctocpp/browser_host_ctocpp.cc",
        "ctocpp/callback_ctocpp.cc",
        "ctocpp/command_line_ctocpp.cc",
        "ctocpp/context_menu_params_ctocpp.cc",
        "ctocpp/cookie_manager_ctocpp.cc",
        "ctocpp/dictionary_value_ctocpp.cc",
        "ctocpp/domdocument_ctocpp.cc",
        "ctocpp/domnode_ctocpp.cc",
        "ctocpp/download_item_callback_ctocpp.cc",
        "ctocpp/download_item_ctocpp.cc",
        "ctocpp/drag_data_ctocpp.cc",
        "ctocpp/file_dialog_callback_ctocpp.cc",
        "ctocpp/frame_ctocpp.cc",
        "ctocpp/image_ctocpp.cc",
        "ctocpp/jsdialog_callback_ctocpp.cc",
        "ctocpp/list_value_ctocpp.cc",
        "ctocpp/media_access_callback_ctocpp.cc",
        "ctocpp/media_route_ctocpp.cc",
        "ctocpp/media_router_ctocpp.cc",
        "ctocpp/media_sink_ctocpp.cc",
        "ctocpp/media_source_ctocpp.cc",
        "ctocpp/menu_model_ctocpp.cc",
        "ctocpp/navigation_entry_ctocpp.cc",
        "ctocpp/permission_prompt_callback_ctocpp.cc",
        "ctocpp/post_data_ctocpp.cc",
        "ctocpp/post_data_element_ctocpp.cc",
        "ctocpp/preference_manager_ctocpp.cc",
        "ctocpp/preference_registrar_ctocpp.cc",
        "ctocpp/print_dialog_callback_ctocpp.cc",
        "ctocpp/print_job_callback_ctocpp.cc",
        "ctocpp/print_settings_ctocpp.cc",
        "ctocpp/process_message_ctocpp.cc",
        "ctocpp/registration_ctocpp.cc",
        "ctocpp/request_context_ctocpp.cc",
        "ctocpp/request_ctocpp.cc",
        "ctocpp/resource_bundle_ctocpp.cc",
        "ctocpp/resource_read_callback_ctocpp.cc",
        "ctocpp/resource_skip_callback_ctocpp.cc",
        "ctocpp/response_ctocpp.cc",
        "ctocpp/run_context_menu_callback_ctocpp.cc",
        "ctocpp/run_quick_menu_callback_ctocpp.cc",
        "ctocpp/scheme_registrar_ctocpp.cc",
        "ctocpp/select_client_certificate_callback_ctocpp.cc",
        "ctocpp/server_ctocpp.cc",
        "ctocpp/shared_memory_region_ctocpp.cc",
        "ctocpp/shared_process_message_builder_ctocpp.cc",
        "ctocpp/sslinfo_ctocpp.cc",
        "ctocpp/sslstatus_ctocpp.cc",
        "ctocpp/stream_reader_ctocpp.cc",
        "ctocpp/stream_writer_ctocpp.cc",
        "ctocpp/task_manager_ctocpp.cc",
        "ctocpp/task_runner_ctocpp.cc",
        "ctocpp/thread_ctocpp.cc",
        "ctocpp/unresponsive_process_callback_ctocpp.cc",
        "ctocpp/urlrequest_ctocpp.cc",
        "ctocpp/v8_context_ctocpp.cc",
        "ctocpp/v8_exception_ctocpp.cc",
        "ctocpp/v8_stack_frame_ctocpp.cc",
        "ctocpp/v8_stack_trace_ctocpp.cc",
        "ctocpp/v8_value_ctocpp.cc",
        "ctocpp/value_ctocpp.cc",
        "ctocpp/waitable_event_ctocpp.cc",
        "ctocpp/x509_cert_principal_ctocpp.cc",
        "ctocpp/x509_certificate_ctocpp.cc",
        "ctocpp/xml_reader_ctocpp.cc",
        "ctocpp/zip_reader_ctocpp.cc",
        "ctocpp/views/box_layout_ctocpp.cc",
        "ctocpp/views/browser_view_ctocpp.cc",
        "ctocpp/views/button_ctocpp.cc",
        "ctocpp/views/display_ctocpp.cc",
        "ctocpp/views/fill_layout_ctocpp.cc",
        "ctocpp/views/label_button_ctocpp.cc",
        "ctocpp/views/layout_ctocpp.cc",
        "ctocpp/views/menu_button_ctocpp.cc",
        "ctocpp/views/menu_button_pressed_lock_ctocpp.cc",
        "ctocpp/views/overlay_controller_ctocpp.cc",
        "ctocpp/views/panel_ctocpp.cc",
        "ctocpp/views/scroll_view_ctocpp.cc",
        "ctocpp/views/textfield_ctocpp.cc",
        "ctocpp/views/view_ctocpp.cc",
        "ctocpp/views/window_ctocpp.cc",
        "wrapper/cef_byte_read_handler.cc",
        "wrapper/cef_closure_task.cc",
        "wrapper/cef_message_router.cc",
        "wrapper/cef_message_router_utils.cc",
        "wrapper/cef_resource_manager.cc",
        "wrapper/cef_scoped_temp_dir.cc",
        "wrapper/cef_stream_resource_handler.cc",
        "wrapper/cef_xml_object.cc",
        "wrapper/cef_zip_archive.cc",
        "wrapper/libcef_dll_wrapper.cc",
        "wrapper/libcef_dll_wrapper2.cc",
    ];

    if cfg!(target_os = "macos") {
        sources.extend([
            "wrapper/cef_library_loader_mac.mm",
            "wrapper/libcef_dll_dylib.cc",
        ]);
    }

    let sources = sources
        .into_iter()
        .map(|path| cef_root.join("libcef_dll").join(path));

    Build::new()
        .cpp(true)
        .std("c++17")
        .cargo_warnings(false)
        .include(cef_root)
        .define("NOMINMAX", None)
        .define("WRAPPING_CEF_SHARED", None)
        .files(sources)
        .compile("cef-dll-wrapper");
}

fn build_wef_sys(cef_root: &Path) {
    println!("cargo::rerun-if-changed=cpp");

    #[allow(unused_mut)]
    let mut sources = vec![
        "cpp/wef.cpp",
        "cpp/client.cpp",
        "cpp/dirty_rect.cpp",
        "cpp/frame.cpp",
        "cpp/file_dialog.cpp",
        "cpp/cursor.cpp",
        "cpp/js_dialog.cpp",
        "cpp/query.cpp",
        "cpp/external_pump.cpp",
    ];

    if cfg!(target_os = "windows") {
        sources.extend(["cpp/external_pump_win.cpp"]);
    } else if cfg!(target_os = "macos") {
        sources.extend([
            "cpp/load_library.cpp",
            "cpp/sandbox_context.cpp",
            "cpp/external_pump_mac.mm",
        ]);
    } else if cfg!(target_os = "linux") {
        sources.extend(["cpp/external_pump_linux.cpp"]);
    }

    let mut build = Build::new();

    if cfg!(target_os = "linux") {
        build.includes(
            pkg_config::probe_library("glib-2.0")
                .unwrap_or_else(|err| panic!("failed to find glib-2.0: {}", err))
                .include_paths,
        );
    }

    build
        .cpp(true)
        .std("c++17")
        .cargo_warnings(false)
        .files(sources)
        .include(cef_root)
        .define("NOMINMAX", None)
        .compile("wef-sys");
}
