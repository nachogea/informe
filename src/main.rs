mod artifact;
mod data_visuals;
mod export;
mod ipc;
mod loader;
mod preview;
mod render;
mod review;
mod review_cli;
mod review_panel;
mod viewer;
mod workspace;

use gpui::*;
use std::path::PathBuf;
use workspace::Workspace;

actions!(informe, [Quit, OpenDocument, CloseDocument, FindDocument]);

fn default_artifact_path() -> PathBuf {
    if let Some(resources) = std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.parent()?
                .parent()
                .map(|contents| contents.join("Resources/dashboard.json"))
        })
        .filter(|path| path.is_file())
    {
        return resources;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/viewer/dashboard.json")
}

fn main() {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    if let Some(result) = review_cli::run(&arguments) {
        if let Err(e) = result {
            eprintln!("Review failed: {e}");
            std::process::exit(1);
        }
        return;
    }
    if arguments.first().is_some_and(|a| a == "export-png") {
        if arguments.len() != 3 {
            eprintln!("Usage: informe export-png artifact.json output.png");
            std::process::exit(2);
        }
        let result = std::fs::read(&arguments[1])
            .map_err(|e| e.to_string())
            .and_then(|b| artifact::Artifact::parse_document(&b, &PathBuf::from(&arguments[1])))
            .and_then(|a| export::png(&a.root))
            .and_then(|bytes| std::fs::write(&arguments[2], bytes).map_err(|e| e.to_string()));
        if let Err(e) = result {
            eprintln!("Export failed: {e}");
            std::process::exit(1);
        }
        println!("Saved {}", PathBuf::from(&arguments[2]).display());
        return;
    }

    let mut serve = false;
    let mut check = false;
    let mut detached = false;
    let mut ready_file = None;
    let mut args = std::env::args_os().skip(1);
    let mut path = None;
    while let Some(arg) = args.next() {
        if arg == "--version" {
            println!("informe {}", env!("CARGO_PKG_VERSION"));
            return;
        } else if arg == "schema" {
            print!("{}", include_str!("../schema/artifact-v1.json"));
            return;
        } else if arg == "capabilities" {
            println!(
                "{}",
                serde_json::json!({
                    "version": env!("CARGO_PKG_VERSION"), "artifact_versions": [1], "document_formats": ["json", "md", "markdown"], "find": "matching_blocks", "section_navigation": true, "table_sort": true,
                    "nodes": ["column", "row", "card", "heading", "text", "markdown", "metric", "callout", "divider", "button", "chart", "table"],
                    "chart_kinds": ["bar"], "actions": ["noop"],
                    "reload_poll_ms": 250, "copy": ["text", "document", "component_png", "table_tsv"], "exports": ["png", "csv"], "drag_text_selection": true, "document_tabs": true, "open_ack": "gpui_after_frame", "review": {"comments": ["node", "quoted_text"], "feedback": "revision_bound_json", "wait": true, "guarded_revision": true}
                })
            );
            return;
        } else if arg == "--serve" {
            serve = true;
        } else if arg == "open" {
            detached = true;
        } else if arg == "--ready-file" {
            ready_file = args.next().map(PathBuf::from);
            if ready_file.is_none() {
                eprintln!("--ready-file needs a path");
                std::process::exit(2);
            }
        } else if arg == "--check" {
            check = true;
        } else if arg == "--help" || arg == "-h" {
            println!(
                "Usage: informe [open | --check] [document.json|document.md]\n       informe --version | schema | capabilities\n       informe --ready-file marker.json artifact.json\n       informe export-png artifact.json output.png\n       informe review artifact.json [--wait --timeout 300] [--after FEEDBACK_ID]\n       informe comment artifact.json --node ID --text COMMENT\n       informe submit artifact.json\n       informe apply-revision artifact.json candidate.json --expect HASH\n\nDefaults to examples/viewer/dashboard.json.\n--check validates the file without opening a window."
            );
            return;
        } else if arg.to_string_lossy().starts_with('-') || path.is_some() {
            eprintln!(
                "Usage: informe [open | --check] [document.json|document.md]\n       informe --version | schema | capabilities\n       informe --ready-file marker.json artifact.json"
            );
            std::process::exit(2);
        } else {
            path = Some(PathBuf::from(arg));
        }
    }
    let path = path.unwrap_or_else(default_artifact_path);
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .expect("current directory")
            .join(path)
    };
    if check {
        let result = std::fs::read(&path)
            .map_err(|e| e.to_string())
            .and_then(|bytes| artifact::Artifact::parse_document(&bytes, &path));
        match result {
            Ok(_) => println!("Valid artifact: {}", path.display()),
            Err(error) => {
                eprintln!("{}: {error}", path.display());
                std::process::exit(1);
            }
        }
        return;
    }
    if detached {
        match ipc::open(&path) {
            Ok(()) => println!("Opened {}", path.display()),
            Err(error) => {
                eprintln!("Open failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    let mut server = None;
    let mut requests = None;
    if ready_file.is_none() {
        match ipc::Server::bind(&ipc::runtime_dir()) {
            Ok(Some((host, rx))) => {
                server = Some(host);
                requests = Some(rx);
            }
            Ok(None) => {
                if !serve && let Err(e) = ipc::open(&path) {
                    eprintln!("Open failed: {e}");
                    std::process::exit(1);
                }
                return;
            }
            Err(e) => {
                eprintln!("Could not start document service: {e}");
                std::process::exit(1);
            }
        }
    }
    let initial_path = if serve { None } else { Some(path.clone()) };
    gpui_platform::application()
        .with_quit_mode(QuitMode::LastWindowClosed)
        .run(move |cx| {
            gpui_base::init(cx);
            gpui_base::Root::register_plugin::<review_panel::ReviewSelection>(cx, |_, _| {
                review_panel::ReviewSelection
            });
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            cx.set_menus([
                Menu::new("Informe").items([
                    MenuItem::os_submenu("Services", SystemMenuType::Services),
                    MenuItem::separator(),
                    MenuItem::action("Quit Informe", Quit),
                ]),
                Menu::new("File").items([
                    MenuItem::action("Open…", OpenDocument),
                    MenuItem::action("Close Document", CloseDocument),
                ]),
                Menu::new("Edit").items([MenuItem::action("Copy", gpui_base::input::Copy)]),
                Menu::new("View").items([MenuItem::action("Find…", FindDocument)]),
            ]);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(1100.), px(760.)), cx)),
                    titlebar: Some(TitlebarOptions {
                        title: Some(
                            format!(
                                "Informe — {}",
                                path.file_name().unwrap_or_default().to_string_lossy()
                            )
                            .into(),
                        ),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(|cx| Workspace::new(initial_path, requests, ready_file, cx));
                    cx.new(|cx| gpui_base::Root::new(view, window, cx))
                },
            )
            .expect("open preview window");
            cx.activate(true);
        });
    drop(server);
}
