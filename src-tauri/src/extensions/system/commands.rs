//! The four commands. Each is small on purpose: the work is the platform's and
//! the shape is the protocol's.

use std::sync::Arc;

use crate::extensions::applications::IconCache;
use crate::invocation::{Command, InvocationContext};
use crate::platform::SystemControl;

use super::{confirmation, report, running_list};

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
    fn invoke(&self, ctx: &InvocationContext) {
        ctx.push_view(running_list(self.0.running_apps(), &self.1));
    }
}
