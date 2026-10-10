use super::Workspace;
use glim_core::GitRequest;
use gpui_kit::*;
use std::path::PathBuf;
impl Workspace {
    pub(crate) fn has_running_git(&self, cx: &App) -> bool {
        self.changes.as_ref().is_some_and(|v| v.read(cx).is_busy())
    }
    pub(crate) fn unlock_git_and_refresh(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_git_editing_locked(false, cx);
        self.refresh(window, cx);
    }

    pub(crate) fn set_git_editing_locked(&mut self, locked: bool, cx: &mut Context<Self>) {
        self.git_edit_locked = locked;
        if let Some(changes) = &self.changes {
            changes.update(cx, |v, cx| v.set_external_busy(locked, cx));
        }
        if locked {
            self.refresh_task = None;
        }
        for reader in &mut self.tabs {
            reader.set_editing_locked(locked, cx);
        }
        cx.notify();
    }
    pub(super) fn run_git(&mut self, root: PathBuf, request: GitRequest, cx: &mut Context<Self>) {
        let Some(changes) = self.changes.clone() else {
            return;
        };
        if self.git_busy || changes.read(cx).is_busy() || self.git_edit_locked {
            return;
        }
        if request.operation.changes_worktree() {
            if self.has_unsaved() || self.saving.is_some() {
                self.error = Some(
                    "Save all drafts before changing branches or updating the working tree.".into(),
                );
                cx.notify();
                return;
            }
            if let Err(error) = crate::app::lock_git_editors(cx.entity().entity_id(), cx) {
                self.error = Some(error.to_string().into());
                cx.notify();
                return;
            }
            self.set_git_editing_locked(true, cx);
        }
        self.git_busy = true;
        self.refresh_task = None;
        changes.update(cx, |v, cx| v.run_operation(root, request, cx));
        cx.notify();
    }
    pub(super) fn finish_git(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.git_busy = false;
        if self.git_edit_locked {
            self.set_git_editing_locked(false, cx);
            crate::app::unlock_git_editors(cx.entity().entity_id(), cx);
        }
        self.refresh(window, cx);
        cx.notify();
    }
}
