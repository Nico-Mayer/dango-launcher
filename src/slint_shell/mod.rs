//! The native shell: Slint draws the launcher and runs the tray, in the Dango
//! process, with no browser engine.

pub mod generated {
    slint::include_modules!();
}
mod icons;
mod instance;
mod keys;
mod render;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::app::{self, Core, Startup};
use crate::extensions::updates::DisabledSource;
use crate::platform::{self, LauncherWindow as PlatformWindow, Permission, RawWindowHandle};
use crate::shell::{self, Shell, TrayCommand, TrayStatus};
use crate::ui::{self, Intent};
use generated::{LauncherWindow, Tray};
use render::Renderer;

thread_local! {
    /// Lives on the UI thread, as the component does.
    static RENDERER: RefCell<Option<Rc<RefCell<Renderer>>>> = const { RefCell::new(None) };
}

struct SlintShell {
    window: slint::Weak<LauncherWindow>,
    tray: slint::Weak<Tray>,
    /// Attached once the native window exists, which is after its first show.
    platform: Mutex<Option<Box<dyn PlatformWindow>>>,
}

impl SlintShell {
    fn handle(window: &LauncherWindow) -> Option<RawWindowHandle> {
        use raw_window_handle::HasWindowHandle;
        Some(
            window
                .window()
                .window_handle()
                .window_handle()
                .ok()?
                .as_raw(),
        )
    }
}

impl Shell for SlintShell {
    fn publish(&self, snapshot: &ui::Snapshot) {
        let window = self.window.clone();
        let snapshot = snapshot.clone();
        let _ = slint::invoke_from_event_loop(move || {
            let Some(window) = window.upgrade() else {
                return;
            };
            RENDERER.with(|renderer| {
                if let Some(renderer) = renderer.borrow().as_ref() {
                    renderer.borrow_mut().render(&window, snapshot);
                }
            });
        });
    }

    fn show(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let mut platform = self.platform.lock().unwrap();
        if let Some(platform) = platform.as_mut() {
            platform.will_show();
            if let Some(handle) = Self::handle(&window) {
                platform.position_on_active_display(handle);
            }
        }
        if let Err(error) = window.show() {
            eprintln!("[dango] could not show the launcher: {error}");
            return;
        }
        let Some(handle) = Self::handle(&window) else {
            return;
        };
        let attached = platform.get_or_insert_with(|| {
            let attached = platform::launcher_window(handle);
            attached.position_on_active_display(handle);
            attached
        });
        attached.did_show(handle);
    }

    fn hide(&self, restore_focus: bool) {
        if let Some(window) = self.window.upgrade() {
            let _ = window.hide();
        }
        if restore_focus {
            if let Some(platform) = self.platform.lock().unwrap().as_mut() {
                platform.restore_previous_focus();
            }
        }
    }

    fn on_main_thread(&self, work: Box<dyn FnOnce() + Send>) {
        if slint::invoke_from_event_loop(work).is_err() {
            eprintln!("[dango] could not reach the main thread");
        }
    }

    fn set_tray(&self, tray: &TrayStatus) {
        let handle = self.tray.clone();
        let tray = tray.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(component) = handle.upgrade() {
                show_tray(&component, &tray);
            }
        });
    }

    fn quit(&self) {
        let _ = slint::quit_event_loop();
    }
}

fn show_tray(component: &Tray, tray: &TrayStatus) {
    let lines: Vec<SharedString> = tray
        .items()
        .into_iter()
        .filter(|item| item.command.is_none())
        .map(|item| item.text.into())
        .collect();
    component.set_lines(ModelRc::from(Rc::new(VecModel::from(lines))));
    component.set_grant_accessibility(tray.missing.contains(&Permission::Accessibility));
    component.set_grant_input_monitoring(tray.missing.contains(&Permission::InputMonitoring));
}

fn select_backend() -> Result<(), slint::PlatformError> {
    let selector = slint::BackendSelector::new().backend_name("winit".into());
    // An accessory app has no dock icon and never becomes the active app by
    // being shown, which is what lets the panel float over fullscreen spaces.
    #[cfg(target_os = "macos")]
    let selector = selector.with_winit_event_loop_builder({
        use winit::platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS};
        let mut builder =
            winit::event_loop::EventLoop::<slint::winit_030::SlintEvent>::with_user_event();
        builder.with_activation_policy(ActivationPolicy::Accessory);
        builder
    });
    selector.select()
}

pub fn run() {
    let Some(_instance) = instance::claim() else {
        if let Err(error) = instance::signal(instance::NAME) {
            eprintln!("[dango] another Dango runs but did not answer: {error}");
        }
        return;
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    select_backend().expect("the winit backend");
    let window = LauncherWindow::new().expect("the launcher window");
    let tray = Tray::new().expect("the tray icon");
    tray.set_image(tray_icon());
    tray.set_hint(shell::tooltip().into());

    let renderer = Rc::new(RefCell::new(Renderer::new(&window)));
    RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer.clone()));

    let shell = Arc::new(SlintShell {
        window: window.as_weak(),
        tray: tray.as_weak(),
        platform: Mutex::new(None),
    });

    // The hotkeys and the tray need the event loop running on this thread.
    let handle = runtime.handle().clone();
    let weak_window = window.as_weak();
    let weak_tray = tray.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        let core = app::start(Startup {
            shell: shell.clone(),
            runtime: handle,
            updates: Arc::new(DisabledSource),
        });
        let (Some(window), Some(tray)) = (weak_window.upgrade(), weak_tray.upgrade()) else {
            return;
        };
        connect(&window, &tray, &core, renderer);
        {
            let core = core.clone();
            if let Err(error) = instance::listen(instance::NAME, move || core.show(None)) {
                eprintln!("[dango] a second launch cannot reach this one: {error}");
            }
        }
        core.dispatch(Intent::Ready);
    });

    if let Err(error) = slint::run_event_loop_until_quit() {
        eprintln!("[dango] the event loop stopped: {error}");
    }
    drop(window);
    drop(tray);
}

fn connect(
    window: &LauncherWindow,
    tray: &Tray,
    core: &Arc<Core>,
    renderer: Rc<RefCell<Renderer>>,
) {
    let send = {
        let core = core.clone();
        Rc::new(move |intent| core.dispatch(intent)) as Rc<dyn Fn(Intent)>
    };
    render::connect(window, renderer.clone(), send);

    {
        let core = core.clone();
        platform::watch_focus_loss(Box::new(move || core.hide_on_focus_loss()));
    }
    {
        let core = core.clone();
        // The frame is drawn while Slint handles this redraw. A rendering
        // notifier would be exact, but the software renderer, which Slint
        // falls back to without OpenGL 2, has none.
        window.window().on_winit_window_event(move |_, event| {
            match event {
                winit::event::WindowEvent::Focused(false) => core.hide_on_focus_loss(),
                winit::event::WindowEvent::RedrawRequested => {
                    if let Some(probe) = renderer.borrow_mut().painted() {
                        core.dispatch(Intent::Painted { probe });
                    }
                }
                _ => {}
            }
            EventResult::Propagate
        });
    }

    let tray_core = core.clone();
    tray.on_toggle(move || tray_core.tray_command(TrayCommand::Toggle));
    let tray_core = core.clone();
    tray.on_quit(move || tray_core.tray_command(TrayCommand::Quit));
    let tray_core = core.clone();
    tray.on_grant(move |permission| {
        let permission = match permission {
            0 => Permission::Accessibility,
            _ => Permission::InputMonitoring,
        };
        tray_core.tray_command(TrayCommand::Grant(permission));
    });
}

fn tray_icon() -> slint::Image {
    decode_png(include_bytes!("../../icons/32x32.png")).unwrap_or_default()
}

fn decode_png(bytes: &[u8]) -> Option<slint::Image> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::ALPHA);
    let mut reader = decoder.read_info().ok()?;
    let mut pixels = vec![0; reader.output_buffer_size()?];
    let frame = reader.next_frame(&mut pixels).ok()?;
    if frame.color_type != png::ColorType::Rgba || frame.bit_depth != png::BitDepth::Eight {
        return None;
    }
    let buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
        &pixels[..frame.buffer_size()],
        frame.width,
        frame.height,
    );
    Some(slint::Image::from_rgba8(buffer))
}
