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

#[cfg(test)]
pub(crate) fn render_headless(snapshot: crate::ui::Snapshot) -> generated::LauncherWindow {
    i_slint_backend_testing::init_no_event_loop();
    let ui = generated::LauncherWindow::new().unwrap();
    ui.window().set_size(slint::LogicalSize::new(720.0, 480.0));
    ui.show().unwrap();
    let mut renderer = render::Renderer::new(&ui);
    render::publish(&mut renderer, &ui, snapshot, |_| {});
    ui
}

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use slint::winit_030::{winit, EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::app::{self, Core, Startup};
use crate::config::LauncherDisplay;
use crate::extensions::updates;
use crate::platform::{self, LauncherWindow as PlatformWindow, Permission, RawWindowHandle};
use crate::shell::{Shell, TrayCommand, TrayStatus};
use crate::ui::{self, Intent};
use generated::{LauncherWindow, NoticeWindow, Tray};
use render::notice::Change;
use render::Renderer;

thread_local! {
    /// Lives on the UI thread, as the component does.
    static RENDERER: RefCell<Option<Rc<RefCell<Renderer>>>> = const { RefCell::new(None) };
    /// The id of the notice on screen.
    static NOTICE_SHOWN: Cell<Option<u64>> = const { Cell::new(None) };
    /// Set while the notice window is created, so the attributes hook can tell it
    /// from the launcher: the hook runs before Slint gives a window its title.
    static CREATING_NOTICE: Cell<bool> = const { Cell::new(false) };
}

type Platform = Arc<Mutex<Box<dyn PlatformWindow>>>;

struct SlintShell {
    window: slint::Weak<LauncherWindow>,
    tray: slint::Weak<Tray>,
    platform: Platform,
    attached: AtomicBool,
    notice: Notice,
}

#[derive(Clone)]
struct Notice {
    window: slint::Weak<NoticeWindow>,
    platform: Arc<Mutex<Box<dyn platform::NoticeWindow>>>,
    attached: Arc<AtomicBool>,
    display: Arc<Mutex<LauncherDisplay>>,
}

impl SlintShell {
    fn attach_once(&self, platform: &mut dyn PlatformWindow, handle: RawWindowHandle) {
        if !self.attached.swap(true, Ordering::Relaxed) {
            platform.attach(handle);
        }
    }
}

fn handle(window: &slint::Window) -> Option<RawWindowHandle> {
    use raw_window_handle::HasWindowHandle;
    Some(window.window_handle().window_handle().ok()?.as_raw())
}

impl Notice {
    fn update(&self, notice: Option<&ui::snapshot::Notice>) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        match (render::notice::change(NOTICE_SHOWN.get(), notice), notice) {
            (Some(Change::Show), Some(notice)) => {
                render::notice::write(&window, notice);
                if self.show(&window) {
                    NOTICE_SHOWN.set(Some(notice.id));
                }
            }
            (Some(Change::Hide), _) => {
                let _ = window.hide();
                NOTICE_SHOWN.set(None);
            }
            _ => {}
        }
    }

    fn show(&self, window: &NoticeWindow) -> bool {
        let mut platform = self.platform.lock().unwrap();
        // The native window may not exist before the first show.
        let existing = handle(window.window());
        if let Some(handle) = existing {
            self.attach_once(platform.as_mut(), handle);
            platform.place(handle, *self.display.lock().unwrap());
        }
        if let Err(error) = window.show() {
            eprintln!("[dango] could not show the notice: {error}");
            return false;
        }
        window.window().with_winit_window(|window| {
            let _ = window.set_cursor_hittest(false);
        });
        let Some(handle) = handle(window.window()) else {
            return true;
        };
        if existing.is_none() {
            self.attach_once(platform.as_mut(), handle);
            platform.place(handle, *self.display.lock().unwrap());
        }
        platform.did_show(handle);
        true
    }

    fn attach_once(&self, platform: &mut dyn platform::NoticeWindow, handle: RawWindowHandle) {
        if !self.attached.swap(true, Ordering::Relaxed) {
            platform.attach(handle);
        }
    }
}

impl Shell for SlintShell {
    fn publish(&self, snapshot: &ui::Snapshot) {
        let window = self.window.clone();
        let platform = self.platform.clone();
        let notice = self.notice.clone();
        let snapshot = snapshot.clone();
        let _ = slint::invoke_from_event_loop(move || {
            notice.update(snapshot.notice.as_ref());
            let Some(window) = window.upgrade() else {
                return;
            };
            RENDERER.with(|renderer| {
                if let Some(renderer) = renderer.borrow().as_ref() {
                    render::publish(&mut renderer.borrow_mut(), &window, snapshot, |height| {
                        set_launcher_height(&window, &platform, height)
                    });
                }
            });
        });
    }

    fn show(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let mut platform = self.platform.lock().unwrap();
        platform.will_show();
        // The native window may not exist before the first show.
        let existing = handle(window.window());
        if let Some(handle) = existing {
            self.attach_once(platform.as_mut(), handle);
            platform.position_on_active_display(handle, *self.notice.display.lock().unwrap());
        }
        if let Err(error) = window.show() {
            eprintln!("[dango] could not show the launcher: {error}");
            return;
        }
        let Some(handle) = handle(window.window()) else {
            return;
        };
        if existing.is_none() {
            self.attach_once(platform.as_mut(), handle);
            platform.position_on_active_display(handle, *self.notice.display.lock().unwrap());
        }
        platform.did_show(handle);
    }

    fn hide(&self, restore_focus: bool) {
        if let Some(window) = self.window.upgrade() {
            let _ = window.hide();
        }
        RENDERER.with(|renderer| {
            if let Some(renderer) = renderer.borrow().as_ref() {
                renderer.borrow_mut().forget_images();
            }
        });
        if restore_focus {
            self.platform.lock().unwrap().restore_previous_focus();
        }
    }

    fn set_display(&self, display: LauncherDisplay) {
        *self.notice.display.lock().unwrap() = display;
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

fn set_launcher_height(window: &LauncherWindow, platform: &Platform, height: f32) {
    let shell = window.global::<generated::Shell>();
    if shell.get_height() == height {
        return;
    }
    let platform = platform.lock().unwrap();
    let Some(handle) = handle(window.window()) else {
        return;
    };
    // Slint resizes the window itself when the component's height changes,
    // which on macOS moves the top edge, so only a platform that keeps it
    // lets the component follow. The component goes first: a native resize
    // to a height it does not allow is sized straight back.
    if !platform.keeps_top_edge() {
        return;
    }
    shell.set_height(height);
    platform.set_height(handle, height.into());
}

fn show_tray(component: &Tray, tray: &TrayStatus) {
    let lines: Vec<SharedString> = tray.lines().into_iter().map(Into::into).collect();
    component.set_lines(ModelRc::from(Rc::new(VecModel::from(lines))));
    component.set_grant_accessibility(tray.missing.contains(&Permission::Accessibility));
    component.set_grant_input_monitoring(tray.missing.contains(&Permission::InputMonitoring));
}

fn select_backend() -> Result<(), slint::PlatformError> {
    let selector = slint::BackendSelector::new()
        .backend_name("winit".into())
        .with_winit_window_attributes_hook(|attributes| {
            if CREATING_NOTICE.get() {
                attributes.with_active(false)
            } else {
                attributes
            }
        });
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
    let Some(instance) = instance::claim() else {
        if let Err(error) = instance::signal(instance::NAME) {
            eprintln!("[dango] another Dango runs but did not answer: {error}");
        }
        return;
    };
    // Released before a restart into an update, or the new Dango would find
    // this one still running and exit.
    let instance = Arc::new(Mutex::new(Some(instance)));

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    select_backend().expect("the winit backend");
    let window = LauncherWindow::new().expect("the launcher window");
    let notice = create_notice();
    let tray = Tray::new().expect("the tray icon");
    tray.set_image(tray_icon());
    tray.set_hint("Dango".into());

    let renderer = Rc::new(RefCell::new(Renderer::new(&window)));
    RENDERER.with(|slot| *slot.borrow_mut() = Some(renderer.clone()));

    let shell = Arc::new(SlintShell {
        window: window.as_weak(),
        tray: tray.as_weak(),
        platform: Arc::new(Mutex::new(platform::launcher_window())),
        attached: AtomicBool::new(false),
        notice: Notice {
            window: notice.as_weak(),
            platform: Arc::new(Mutex::new(platform::notice_window())),
            attached: Arc::new(AtomicBool::new(false)),
            display: Default::default(),
        },
    });

    // The hotkeys and the tray need the event loop running on this thread.
    let handle = runtime.handle().clone();
    let held = instance.clone();
    let weak_window = window.as_weak();
    let weak_tray = tray.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        let core = app::start(Startup {
            shell: shell.clone(),
            runtime: handle,
            updates: updates::production(Box::new(move |app| {
                held.lock().unwrap().take();
                platform::relaunch(app);
            })),
        });
        let (Some(window), Some(tray)) = (weak_window.upgrade(), weak_tray.upgrade()) else {
            return;
        };
        connect(&window, &tray, &core, renderer);
        {
            let core = core.clone();
            if let Err(error) = instance::listen(instance::NAME, move || core.show(None, true)) {
                eprintln!("[dango] a second launch cannot reach this one: {error}");
            }
        }
        {
            let core = core.clone();
            platform::watch_reopen(Box::new(move || core.show(None, true)));
        }
        core.dispatch(Intent::Ready);
    });

    if let Err(error) = slint::run_event_loop_until_quit() {
        eprintln!("[dango] the event loop stopped: {error}");
    }
    drop(window);
    drop(notice);
    drop(tray);
}

/// Slint makes a window's adapter, and with it the winit attributes, on first
/// use rather than in `new`, so the window is touched while the flag is set.
fn create_notice() -> NoticeWindow {
    CREATING_NOTICE.set(true);
    let notice = NoticeWindow::new().expect("the notice window");
    let _ = notice.window().scale_factor();
    CREATING_NOTICE.set(false);
    notice
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
                winit::event::WindowEvent::Focused(false) if !platform::focus_lent() => {
                    core.hide_on_focus_loss()
                }
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
