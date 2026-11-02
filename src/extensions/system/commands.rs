//! The five commands. Each is small on purpose: the work is the platform's and
//! the shape is the protocol's.

use std::sync::Arc;

use crate::config::FileConfig;
use crate::icon_cache::IconCache;
use crate::invocation::{Command, InvocationContext};
use crate::platform::SystemControl;

use super::{app_icons, confirmation, render_app_icon, report, running_list, OpenFile};

pub const CREATE_FAILED: &str = "Couldn't create the config file";
pub const OPEN_FAILED: &str = "Couldn't open the config file";

pub struct Lock(pub Arc<dyn SystemControl>);

impl Command for Lock {
    fn invoke(&self, ctx: &InvocationContext) {
        report(ctx, self.0.lock());
    }
}

pub struct Sleep(pub Arc<dyn SystemControl>);

impl Command for Sleep {
    fn invoke(&self, ctx: &InvocationContext) {
        report(ctx, self.0.sleep());
    }
}

pub struct EmptyTrash(pub Arc<dyn SystemControl>);

impl Command for EmptyTrash {
    fn invoke(&self, ctx: &InvocationContext) {
        match self.0.trash_count() {
            // Nothing to lose means nothing to confirm.
            Ok(0) => ctx.succeed(),
            Ok(count) => ctx.push_view(confirmation(count)),
            Err(error) => ctx.fail(error.to_string()),
        }
    }
}

pub struct QuitApplication(pub Arc<dyn SystemControl>, pub Arc<IconCache>);

impl Command for QuitApplication {
    /// Shows the list before rendering missing icons, then shows it again.
    fn invoke(&self, ctx: &InvocationContext) {
        let apps = self.0.running_apps();
        ctx.push_view(running_list(&apps, &self.1));
        if self
            .1
            .fill(app_icons(&apps), render_app_icon, || ctx.is_cancelled())
        {
            ctx.push_view(running_list(&apps, &self.1));
        }
    }
}

pub struct OpenConfigFile(pub Arc<FileConfig>, pub Arc<dyn OpenFile>);

impl Command for OpenConfigFile {
    fn invoke(&self, ctx: &InvocationContext) {
        let path = match self.0.ensure_file() {
            Ok(path) => path,
            Err(error) => {
                crate::log::append(&format!("config: could not create {error}"));
                return ctx.fail(CREATE_FAILED);
            }
        };
        match self.1.open(&path) {
            Ok(()) => ctx.succeed(),
            Err(error) => {
                crate::log::append(&format!(
                    "config: could not open {}: {error}",
                    path.display()
                ));
                ctx.fail(OPEN_FAILED);
            }
        }
    }
}
