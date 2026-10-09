use super::Workspace;
use gpui_kit::*;

impl Workspace {
    pub(super) fn clone_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let url = self.clone_url.read(cx).value().trim().to_owned();
        if !(url.starts_with("https://") || url.starts_with("ssh://") || url.starts_with("git@")) {
            self.error = Some("Enter an HTTPS or SSH repository URL.".into());
            cx.notify();
            return;
        }
        let selection = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose parent folder for clone".into()),
        });
        self.picker_task = Some(cx.spawn_in(window, async move |view, cx| {
            if let Ok(Ok(Some(paths))) = selection.await
                && let Some(parent) = paths.into_iter().next()
            {
                let _ = view.update_in(cx, |view, window, cx| {
                    view.loading = true;
                    view.error = None;
                    let work = cx.background_executor().spawn(async move {
                        glimpse_services::git::clone_repository(&url, &parent)
                    });
                    view.load_task = Some(cx.spawn_in(window, async move |view, cx| {
                        let result = work.await;
                        let _ = view.update_in(cx, |view, window, cx| {
                            view.loading = false;
                            match result {
                                Ok(path) => view.open_path(path, window, cx),
                                Err(error) => view.error = Some(error.to_string().into()),
                            }
                            cx.notify();
                        });
                    }));
                    cx.notify();
                });
            }
        }));
    }
}
