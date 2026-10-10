use super::Workspace;
use glimpse_services::workspace::FolderSnapshot;
use gpui_kit::*;

pub(super) enum DiscardAction {
    CloseTab(usize),
    CloseWindow,
    Folder(FolderSnapshot),
    Quit,
}
impl Workspace {
    pub fn has_unsaved(&self) -> bool {
        self.tabs.iter().any(|r| r.is_dirty())
    }
    pub fn is_saving(&self) -> bool {
        self.saving.is_some()
    }

    pub(super) fn save_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving.is_some() {
            return;
        }
        let Some(reader) = self.active_reader().filter(|r| r.is_dirty()) else {
            return;
        };
        let Some(editor) = reader.editor() else {
            return;
        };
        let id = editor.entity_id();
        let path = reader.path.clone();
        let expected = reader.snapshot.clone();
        let text = editor.read(cx).value().to_string();
        self.saving = Some(id);
        self.error = None;
        // A pre-save read must not arrive later and replace the just-saved buffer.
        self.refresh_task = None;
        let work = cx.background_executor().spawn(async move {
            let result = glimpse_services::files::save_document(&path, &expected, &text);
            (text, result)
        });
        self.save_task = Some(cx.spawn_in(window, async move |view, cx| {
            let (text, result) = work.await;
            let _ = view.update_in(cx, |v, window, cx| {
                v.saving = None;
                match result {
                    Ok(()) => {
                        if let Some(reader) = v
                            .tabs
                            .iter_mut()
                            .find(|r| r.editor().is_some_and(|s| s.entity_id() == id))
                        {
                            reader.mark_saved(text, cx);
                        }
                        v.refresh(window, cx);
                    }
                    Err(error) => v.error = Some(format!("Save failed: {error:#}").into()),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    pub(super) fn confirm_discard(
        &mut self,
        action: DiscardAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_saving() {
            self.error = Some(
                "Saving is still in progress. Please wait before closing or changing folders."
                    .into(),
            );
            cx.notify();
            return;
        }
        if self.confirm_task.is_some() {
            return;
        }
        let all = matches!(action, DiscardAction::Quit);
        let response = window.prompt(PromptLevel::Warning,
            if all { "Quit with unsaved changes?" } else { "Discard unsaved changes?" },
            Some("Your edits have not been saved. Cancel and press ⌘S to save, or discard them to continue."),
            &["Cancel", if all { "Discard All and Quit" } else { "Discard Changes" }], cx);
        self.confirm_task = Some(cx.spawn_in(window, async move |view, cx| {
            let discard = response.await == Ok(1);
            let _ = view.update_in(cx, |v, window, cx| {
                v.confirm_task = None;
                if discard {
                    v.apply_discard_action(action, window, cx);
                }
            });
        }));
    }
    fn apply_discard_action(
        &mut self,
        action: DiscardAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            DiscardAction::CloseTab(index) => self.close_tab_unchecked(index, window, cx),
            DiscardAction::CloseWindow => window.remove_window(),
            DiscardAction::Folder(folder) => self.replace_folder(folder, window, cx),
            DiscardAction::Quit => {
                if self.is_saving() || crate::app::any_other_saving(cx.entity().entity_id(), cx) {
                    self.error =
                        Some("A window is still saving. Please wait before quitting.".into());
                    cx.notify();
                } else {
                    cx.quit();
                }
            }
        }
    }
    pub fn request_quit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.confirm_discard(DiscardAction::Quit, window, cx);
    }
    pub(super) fn request_close_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.has_unsaved() || self.is_saving() {
            self.confirm_discard(DiscardAction::CloseWindow, window, cx);
        } else {
            window.remove_window();
        }
    }
    pub(super) fn receive_folder(
        &mut self,
        folder: FolderSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_unsaved() || self.is_saving() {
            self.confirm_discard(DiscardAction::Folder(folder), window, cx);
        } else {
            self.replace_folder(folder, window, cx);
        }
    }
    fn replace_folder(
        &mut self,
        folder: FolderSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tabs.clear();
        self.active = None;
        self.history.clear();
        self.history_cursor = None;
        self.sidebar = super::Sidebar::Files;
        self.install_folder(folder, window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::Workspace;
    use gpui_kit::{
        AppContext, Bounds, Point, TestAppContext, WindowBounds, WindowOptions, px, size,
        test::TestWindowExt,
    };

    #[gpui_kit::test]
    fn editing_save_refresh_and_close_keep_the_draft_safe(cx: &mut TestAppContext) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, "original\n").unwrap();
        cx.update(|cx| {
            gpui_kit::init(cx);
            cx.bind_keys([gpui_kit::KeyBinding::new(
                "cmd-s",
                crate::app::actions::SaveFile,
                None,
            )]);
        });
        let (handle, view) = cx.update(|cx| {
            gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds {
                        origin: Point::default(),
                        size: size(px(1000.), px(700.)),
                    })),
                    ..Default::default()
                },
                cx,
                |window, cx| cx.new(|cx| Workspace::new(Some(path.clone()), window, cx)),
            )
            .unwrap()
        });
        cx.run_until_parked();
        let editor = cx.update(|cx| view.read(cx).active_reader().unwrap().editor().unwrap());
        cx.update_window(handle, |_, window, cx| {
            window.render_frame(cx);
            window.click(("input", editor.entity_id()), cx);
            window.press("cmd-a", cx);
            window.input("edited\n", cx);
        })
        .unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| view.read(cx).has_unsaved()));
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "original\n");
        cx.update_window(handle, |_, window, cx| window.press("cmd-s", cx))
            .unwrap();
        cx.run_until_parked();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited\n");
        assert!(!cx.update(|cx| view.read(cx).has_unsaved()));
        // Undo survives Save, and is itself an unsaved edit.
        cx.update_window(handle, |_, window, cx| window.press("cmd-z", cx))
            .unwrap();
        cx.run_until_parked();
        assert!(cx.update(|cx| view.read(cx).has_unsaved()));
        std::fs::write(&path, "external\n").unwrap();
        cx.update_window(handle, |_, window, cx| {
            view.update(cx, |v, cx| v.refresh(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_ne!(
            cx.update(|cx| editor.read(cx).value().to_string()),
            "external\n"
        );
        cx.update_window(handle, |_, window, cx| {
            view.update(cx, |v, cx| v.save_active(window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "external\n");
        assert!(cx.update(|cx| view.read(cx).has_unsaved()));
        assert!(cx.update(|cx| view.read(cx).error.is_some()));
        cx.update_window(handle, |_, window, cx| {
            view.update(cx, |v, cx| v.close_tab(0, window, cx))
        })
        .unwrap();
        cx.run_until_parked();
        assert!(cx.has_pending_prompt());
        cx.simulate_prompt_answer("Cancel");
        cx.run_until_parked();
        assert_eq!(cx.update(|cx| view.read(cx).tabs.len()), 1);
        assert!(cx.update(|cx| view.read(cx).has_unsaved()));
    }
}
